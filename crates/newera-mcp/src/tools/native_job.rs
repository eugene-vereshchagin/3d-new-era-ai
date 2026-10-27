//! Bounded native rendering with transport liveness and cooperative cancellation.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, PingRequest, ProgressNotificationParam, ServerRequest,
};
use rmcp::service::{PeerRequestOptions, RequestContext};
use rmcp::{ErrorData, RoleServer};
use tokio_util::sync::CancellationToken;

use super::{NewEraMcp, reply::invalid};

static RENDER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

/// The render running now, for `sessions` to show and `cancel` to stop.
struct Running {
    tool: String,
    started: std::time::Instant,
    cancel: CancellationToken,
    watcher: Arc<Watcher>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

fn running() -> std::sync::MutexGuard<'static, Option<Running>> {
    RUNNING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What the render running now is doing: `{tool, seconds, phase, done,
/// total, threads}`, or `None` when none is.
pub(crate) fn status() -> Option<serde_json::Value> {
    let guard = running();
    let run = guard.as_ref()?;
    let s = run
        .watcher
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Some(serde_json::json!({
        "tool": run.tool,
        "seconds": run.started.elapsed().as_secs(),
        "phase": s.phase,
        "done": s.done,
        "total": s.total,
        "threads": THREADS.load(std::sync::atomic::Ordering::Relaxed),
    }))
}

/// Threads the running photo was given.
pub(crate) static THREADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Stops the render running now; whether there was one.
pub(crate) fn cancel_running() -> bool {
    running().as_ref().is_some_and(|run| {
        run.cancel.cancel();
        true
    })
}

/// Forgets the running render when its supervisor ends, however it ends.
struct Unregister;
impl Drop for Unregister {
    fn drop(&mut self) {
        running().take();
        THREADS.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Default)]
struct State {
    phase: String,
    done: u64,
    total: u64,
    progress: u64,
}
struct Watcher {
    cancel: CancellationToken,
    state: Mutex<State>,
}
impl newera_core::progress::Watcher for Watcher {
    fn step(&self, phase: &str, done: u64, total: u64) {
        let mut s = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = if s.phase == phase { s.done } else { 0 };
        s.progress = s.progress.saturating_add(done.saturating_sub(previous));
        s.done = if s.phase == phase {
            s.done.max(done)
        } else {
            done
        };
        phase.clone_into(&mut s.phase);
        s.total = total;
    }
    fn cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}
struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(super) async fn run(
    server: NewEraMcp,
    request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResponse, ErrorData> {
    // `render_photo cancel=true` stops the render running, instead of queuing.
    if request.name == "render_photo"
        && request
            .arguments
            .as_ref()
            .and_then(|a| a.get("cancel"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        let text = if cancel_running() {
            "cancelled: the render running stops at its next row; nothing in the project changed"
        } else {
            "no render is running"
        };
        return Ok(CallToolResponse::Complete(
            rmcp::model::CallToolResult::success(vec![rmcp::model::ContentBlock::text(text)]),
        ));
    }
    let permit = RENDER.try_acquire().map_err(|_| {
        invalid(
            "A render is already running (`sessions` shows how far it is); wait for it or stop it with render_photo cancel=true.",
        )
    })?;
    let tool = request.name.to_string();
    // A photo shares one slot with heavy plugins (a modeler): the budget is
    // the machine's, not each caller's.
    let slot = if tool == "render_photo" {
        Some(
            newera_core::progress::heavy("render_photo").map_err(|holder| {
                invalid(format!(
                    "{holder} is running, and heavy work runs one at a time; wait for it (a plugin is not in `sessions` and cancel=true does not stop it: it ends by itself or at its timeout_s)"
                ))
            })?,
        )
    } else {
        None
    };
    let handle = tokio::runtime::Handle::current();
    let work_context = context.clone();
    supervise(
        Some(tool),
        context,
        Duration::from_secs(20),
        Duration::from_secs(10),
        move || {
            // Keep the permit until the actual worker has stopped, even if the
            // client abandoned its async request earlier.
            let _permit = permit;
            let _slot = slot;
            handle.block_on(async {
                let call = rmcp::handler::server::tool::ToolCallContext::new(
                    &server,
                    request,
                    work_context,
                );
                server.tool_router.call(call).await
            })
        },
    )
    .await
}

/// A ping on its way, until it is answered or times out; `true` when it
/// could not be sent at all, because the client is gone.
type Ping = std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>;

async fn supervise(
    tool: Option<String>,
    context: RequestContext<RoleServer>,
    heartbeat: Duration,
    response_timeout: Duration,
    work: impl FnOnce() -> Result<CallToolResponse, ErrorData> + Send + 'static,
) -> Result<CallToolResponse, ErrorData> {
    let cancel = context.ct.child_token();
    let _cancel_on_drop = CancelOnDrop(cancel.clone());
    let watcher = Arc::new(Watcher {
        cancel: cancel.clone(),
        state: Mutex::new(State::default()),
    });
    let listener: Arc<dyn newera_core::progress::Watcher> = watcher.clone();
    // Listed until the worker itself stops: a cancelled or abandoned call
    // returns early, and the render still holds its slot until then.
    let unregister = tool.map(|tool| {
        *running() = Some(Running {
            tool,
            started: std::time::Instant::now(),
            cancel: cancel.clone(),
            watcher: watcher.clone(),
        });
        Unregister
    });
    let mut worker = tokio::task::spawn_blocking(move || {
        let _unregister = unregister;
        if listener.cancelled() {
            return Err(invalid(newera_core::progress::CANCELLED));
        }
        let result = newera_core::progress::watched(&listener, work);
        if listener.cancelled() {
            Err(invalid(newera_core::progress::CANCELLED))
        } else {
            result
        }
    });
    let token = context.meta.get_progress_token();
    let mut poll = tokio::time::interval(Duration::from_millis(250));
    let mut next_ping = tokio::time::Instant::now() + heartbeat;
    let mut next_progress = tokio::time::Instant::now();
    let mut ping: Option<Ping> = None;
    loop {
        tokio::select! {
            result = &mut worker => return result.map_err(|e| invalid(format!("The render failed: {e}")))?,
            () = cancel.cancelled() => return Err(invalid(newera_core::progress::CANCELLED)),
            gone = async { ping.as_mut().expect("guarded").await }, if ping.is_some() => {
                if gone { return Err(invalid("The client disconnected; the render was cancelled.")); }
                ping = None;
            }
            _ = poll.tick() => {
                if context.peer.is_transport_closed() { return Err(invalid("The client disconnected; the render was cancelled.")); }
                let now = tokio::time::Instant::now();
                if now >= next_ping && ping.is_none() {
                    // Ping is part of MCP even when no progress token was
                    // supplied, and it keeps an HTTP session from idling out.
                    // An answer is not required: many clients never answer a
                    // ping while a call is open, and a render must not die
                    // for that. A gone client closes the transport or cancels
                    // the call; a photo also stops at its max_s. The ping is
                    // awaited here, alongside the rest, so it goes out on
                    // this call's own stream.
                    let peer = context.peer.clone();
                    ping = Some(Box::pin(async move {
                        let answer = async {
                            peer.send_request_with_option(
                                ServerRequest::PingRequest(PingRequest { method: rmcp::model::PingRequestMethod, extensions: rmcp::model::Extensions::default() }),
                                PeerRequestOptions::with_timeout(response_timeout),
                            ).await?.await_response().await
                        };
                        // Unanswered is fine; only a ping that could not
                        // go out at all says the client is gone.
                        matches!(
                            tokio::time::timeout(response_timeout, answer).await,
                            Ok(Err(rmcp::ServiceError::TransportSend(_) | rmcp::ServiceError::TransportClosed))
                        )
                    }));
                    next_ping = now + heartbeat;
                }
                if now >= next_progress {
                    if let Some(token) = &token {
                        let notification = {
                            let s = watcher.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                            #[allow(clippy::cast_precision_loss)]
                            let progress = s.progress as f64;
                            let message = if s.total == 0 { s.phase.clone() } else { format!("{}: {}/{}", s.phase, s.done, s.total) };
                            ProgressNotificationParam::new(token.clone(), progress).with_message(message)
                        };
                        if !matches!(tokio::time::timeout(response_timeout, context.peer.notify_progress(notification)).await, Ok(Ok(()))) {
                            return Err(invalid("Progress could not be sent; the render was cancelled."));
                        }
                    }
                    // A deleted HTTP session shows only here while its
                    // transport drains: a ping there goes unanswered like
                    // any other, but progress can no longer be sent. So a
                    // short heartbeat (the tests') also checks it often.
                    next_progress = now + heartbeat.min(Duration::from_secs(1));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request};
    use futures_util::StreamExt;
    use rmcp::ServerHandler;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicBool, Ordering};
    use tower::ServiceExt;

    /// Stops when told; with a name, it is registered as the render running.
    #[derive(Clone)]
    struct SlowServer(Arc<AtomicBool>, Option<&'static str>);
    impl ServerHandler for SlowServer {
        async fn call_tool(
            &self,
            _: CallToolRequestParams,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResponse, ErrorData> {
            let stopped = self.0.clone();
            supervise(
                self.1.map(str::to_owned),
                context,
                Duration::from_millis(50),
                Duration::from_millis(400),
                move || {
                    for i in 0..250 {
                        if newera_core::progress::cancelled() {
                            break;
                        }
                        newera_core::progress::step("Test render", i, 250);
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    stopped.store(true, Ordering::SeqCst);
                    Ok(rmcp::model::CallToolResult::success(vec![]).into())
                },
            )
            .await
        }
    }
    async fn post(
        router: &Router,
        session: Option<&str>,
        value: Value,
    ) -> axum::response::Response {
        let mut request = Request::builder()
            .method("POST")
            .uri("/")
            .header("host", "localhost")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream");
        if let Some(session) = session {
            request = request.header("mcp-session-id", session);
        }
        router
            .clone()
            .oneshot(request.body(Body::from(value.to_string())).unwrap())
            .await
            .unwrap()
    }
    async fn setup(name: Option<&'static str>) -> (Router, String, Arc<AtomicBool>) {
        let stopped = Arc::new(AtomicBool::new(false));
        let server = SlowServer(stopped.clone(), name);
        let mut sessions = LocalSessionManager::default();
        sessions.session_config.keep_alive = Some(Duration::from_secs(1));
        let service = StreamableHttpService::new(
            move || Ok(server.clone()),
            Arc::new(sessions),
            StreamableHttpServerConfig::default(),
        );
        let router = Router::new().fallback_service(service);
        let response = post(&router, None, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1"}
        }})).await;
        assert!(response.status().is_success());
        let session = response.headers()["mcp-session-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let _ = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        post(
            &router,
            Some(&session),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        )
        .await;
        (router, session, stopped)
    }
    async fn exercise(mode: &str) {
        let (router, session, stopped) = setup((mode == "stop").then_some("render_photo")).await;
        let mut params = json!({"name":"slow"});
        if mode != "success" {
            params["_meta"] = json!({"progressToken":"progress"});
        }
        let response = post(
            &router,
            Some(&session),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":params}),
        )
        .await;
        let mut stream = response.into_body().into_data_stream();
        let mut buffer = String::new();
        let mut pings = 0;
        let mut progress = 0;
        let mut result = None;
        let mut interrupted = false;
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(6), stream.next())
            .await
            .unwrap()
        {
            buffer.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
            while let Some(end) = buffer.find("\n\n") {
                let event: String = buffer.drain(..end + 2).collect();
                let Some(data) = event.lines().find_map(|l| l.strip_prefix("data: ")) else {
                    continue;
                };
                if data.trim().is_empty() {
                    continue;
                }
                let value: Value = serde_json::from_str(data).unwrap();
                if value["method"] == "ping" {
                    pings += 1;
                    if mode != "unresponsive" {
                        post(
                            &router,
                            Some(&session),
                            json!({"jsonrpc":"2.0","id":value["id"],"result":{}}),
                        )
                        .await;
                    }
                } else if value["method"] == "notifications/progress" {
                    progress += 1;
                    if !interrupted
                        && value["params"]["message"]
                            .as_str()
                            .is_some_and(|m| m.starts_with("Test render"))
                    {
                        if mode == "stop" {
                            // Another call sees it running, and stops it.
                            let running = status().expect("the render is registered");
                            assert_eq!(running["tool"], "render_photo");
                            assert_eq!(running["phase"], "Test render");
                            assert!(cancel_running());
                            interrupted = true;
                        } else if mode == "cancel" {
                            post(&router, Some(&session), json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2,"reason":"test"}})).await;
                            interrupted = true;
                        } else if mode == "disconnect" {
                            let response = router
                                .clone()
                                .oneshot(
                                    Request::builder()
                                        .method("DELETE")
                                        .header("host", "localhost")
                                        .uri("/")
                                        .header("mcp-session-id", &session)
                                        .body(Body::empty())
                                        .unwrap(),
                                )
                                .await
                                .unwrap();
                            assert!(response.status().is_success());
                            interrupted = true;
                        }
                    }
                } else if value["id"] == 2 {
                    result = Some(value);
                }
            }
        }
        if mode == "success" || mode == "unresponsive" {
            assert!(
                pings >= 2,
                "long request must heartbeat without a progress token"
            );
            assert!(
                result.unwrap().get("result").is_some(),
                "a client that never answers a ping still gets its render"
            );
        } else {
            assert!(progress > 0);
            assert!(result.as_ref().is_none_or(|r| r.get("error").is_some()));
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            while !stopped.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker must stop after it ends or its client cancels/disconnects");
        if mode == "stop" {
            // `stopped` is set inside the worker, just before it returns.
            tokio::time::timeout(Duration::from_secs(1), async {
                while status().is_some() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("a finished render is no longer listed");
            assert!(!cancel_running());
        }
    }
    #[tokio::test]
    async fn a_running_render_is_listed_and_stopped_from_another_call() {
        exercise("stop").await;
    }
    #[tokio::test]
    async fn http_long_job_survives_short_session_idle_limit_without_progress_token() {
        exercise("success").await;
    }
    #[tokio::test]
    async fn http_cancel_stops_worker() {
        exercise("cancel").await;
    }
    #[tokio::test]
    async fn http_session_deletion_stops_worker() {
        exercise("disconnect").await;
    }
    #[tokio::test]
    async fn http_client_that_never_answers_pings_gets_its_render() {
        exercise("unresponsive").await;
    }
}
