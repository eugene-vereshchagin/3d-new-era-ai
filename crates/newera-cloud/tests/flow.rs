//! The hosted service end to end, against a real Postgres: an AI client
//! registers, a person signs in with an emailed link and allows it, the
//! client trades the code for tokens, and its calls reach the person's tab.
//!
//! Needs `TEST_DATABASE_URL` (a Postgres the tests may create databases on);
//! without it they say so and pass, so a machine without Postgres still runs
//! the rest of the suite. CI sets it.

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use newera_cloud::{AppState, Config, config::Mail, secret};
use serde_json::{Value, json};
use sha2::Digest as _;

const SITE: &str = "http://127.0.0.1:8790";

/// A fresh database for one test, migrated, and the service on a free port.
async fn service() -> Option<(String, AppState)> {
    let Ok(admin) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set: skipping");
        return None;
    };
    let name = format!("newera_test_{}", secret::id());
    let pool = sqlx::PgPool::connect(&admin)
        .await
        .expect("the test database");
    // The name is our own hex, never input: safe to put in the statement.
    sqlx::query(sqlx::AssertSqlSafe(format!("create database {name}")))
        .execute(&pool)
        .await
        .expect("a database of its own");
    let url = match admin.rsplit_once('/') {
        Some((server, _)) => format!("{server}/{name}"),
        None => panic!("TEST_DATABASE_URL has no database name"),
    };
    let db = newera_cloud::database(&url).await.expect("migrations run");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let config = Config {
        public_url: base.clone(),
        database_url: url,
        site_origins: vec![SITE.to_owned()],
        mail: Mail::Log,
        google: None,
        editor_url: format!("{SITE}/editor/"),
        port: 0,
    };
    let state = AppState::new(config, db);
    let app = newera_cloud::router(&state);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Some((base, state))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

fn location(response: &reqwest::Response) -> String {
    response.headers()["location"].to_str().unwrap().to_owned()
}

fn set_cookie(response: &reqwest::Response) -> String {
    response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn param(url: &str, key: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

/// A hidden field's value in a page.
fn field(html: &str, name: &str) -> String {
    let marker = format!(r#"name="{name}" value=""#);
    let start = html.find(&marker).expect(name) + marker.len();
    html[start..]
        .split('"')
        .next()
        .unwrap()
        .replace("&amp;", "&")
}

/// Signs in through an emailed link: the link is made the way `/login`
/// makes it, and the person presses the button.
async fn sign_in(base: &str, state: &AppState, email: &str, return_to: &str) -> (String, String) {
    let http = client();
    let asked = http
        .post(format!("{base}/login"))
        .form(&[("email", email), ("return_to", return_to)])
        .send()
        .await
        .unwrap();
    assert_eq!(asked.status(), 200, "the link is sent");
    // The log carries the link; the test reads the row instead and swaps in a
    // token it knows (only the hash is stored).
    let token = secret::token();
    sqlx::query("update login_links set token_hash = $1 where email = $2 and used_at is null")
        .bind(secret::hash(&token))
        .bind(email)
        .execute(&state.db)
        .await
        .unwrap();
    let page = http
        .get(format!("{base}/login/link?token={token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(page.status(), 200, "the link opens a page, not a sign-in");
    let signed = http
        .post(format!("{base}/login/link"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(signed.status(), 303);
    let cookie = set_cookie(&signed);
    let again = http
        .post(format!("{base}/login/link"))
        .form(&[("token", token.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 410, "a link works once");
    (cookie, location(&signed))
}

async fn mcp(base: &str, token: &str, body: Value) -> (u16, Value) {
    let response = client()
        .post(format!("{base}/mcp"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or(Value::Null))
}

#[tokio::test]
async fn an_ai_client_is_allowed_in_and_reaches_the_tab() {
    let Some((base, state)) = service().await else {
        return;
    };
    let http = client();

    // Discovery: a call without a token points at the metadata.
    let bare = http
        .post(format!("{base}/mcp"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(bare.status(), 401);
    let challenge = bare.headers()["www-authenticate"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        challenge.contains("/.well-known/oauth-protected-resource/mcp"),
        "{challenge}"
    );
    let resource: Value = http
        .get(format!("{base}/.well-known/oauth-protected-resource/mcp"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resource["resource"], format!("{base}/mcp"));
    let server: Value = http
        .get(format!("{base}/.well-known/oauth-authorization-server"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(server["code_challenge_methods_supported"], json!(["S256"]));

    // Registration.
    let redirect = "http://127.0.0.1/callback";
    let refused = http
        .post(format!("{base}/oauth/register"))
        .json(&json!({"redirect_uris": ["http://evil.example/cb"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        refused.status(),
        400,
        "plain http off this machine is refused"
    );
    let registered: Value = http
        .post(format!("{base}/oauth/register"))
        .json(&json!({"redirect_uris": [redirect], "client_name": "Test <b>client</b>"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let client_id = registered["client_id"].as_str().unwrap().to_owned();

    // The authorization request, with PKCE and a loopback port of its own.
    let verifier = secret::token();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()));
    let callback = "http://127.0.0.1:51234/callback";
    let authorize = format!(
        "{base}/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri={}&code_challenge={challenge}&code_challenge_method=S256&state=xyz&scope=newera&resource={}",
        newera_cloud::login::url_encode(callback),
        newera_cloud::login::url_encode(&format!("{base}/mcp")),
    );
    let first = http.get(&authorize).send().await.unwrap();
    assert_eq!(first.status(), 303, "not signed in yet");
    let to_login = location(&first);
    assert!(
        to_login.starts_with("/login?return_to=%2Foauth%2Fauthorize"),
        "{to_login}"
    );

    // Signing in brings the person back to the request.
    let return_to = param(&format!("{base}{to_login}"), "return_to").unwrap();
    let (cookie, back) = sign_in(&base, &state, "Ana@Example.com", &return_to).await;
    assert!(back.starts_with("/oauth/authorize?"), "{back}");

    // The consent screen names the client (escaped) and the account.
    let consent = http
        .get(format!("{base}{back}"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(consent.contains("Test &lt;b&gt;client&lt;/b&gt;"));
    assert!(consent.contains("Ana@Example.com"));
    let csrf = field(&consent, "csrf");

    // A forged form is refused; the real one gives a code.
    let forged = http
        .post(format!("{base}/oauth/authorize"))
        .header("cookie", &cookie)
        .form(&[
            ("csrf", "nope"),
            ("decision", "allow"),
            ("client_id", &client_id),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(forged.status(), 400);
    let form = [
        ("response_type", "code"),
        ("client_id", client_id.as_str()),
        ("redirect_uri", callback),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", "xyz"),
        ("scope", "newera"),
        ("csrf", csrf.as_str()),
        ("decision", "allow"),
    ];
    let allowed = http
        .post(format!("{base}/oauth/authorize"))
        .header("cookie", &cookie)
        .form(&form)
        .send()
        .await
        .unwrap();
    assert_eq!(allowed.status(), 303);
    let redirected = location(&allowed);
    assert!(redirected.starts_with(callback));
    assert_eq!(param(&redirected, "state").as_deref(), Some("xyz"));
    assert_eq!(param(&redirected, "iss").as_deref(), Some(base.as_str()));
    let code = param(&redirected, "code").unwrap();

    // The code, with the wrong verifier, then the right one; never twice.
    let exchange = |verifier: String| {
        let http = http.clone();
        let (base, code, client_id) = (base.clone(), code.clone(), client_id.clone());
        async move {
            http.post(format!("{base}/oauth/token"))
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("client_id", client_id.as_str()),
                    ("redirect_uri", callback),
                    ("code_verifier", verifier.as_str()),
                ])
                .send()
                .await
                .unwrap()
        }
    };
    let wrong = exchange(secret::token()).await;
    assert_eq!(
        wrong.status(),
        400,
        "a wrong verifier spends the code for nothing"
    );
    // The code was spent by the wrong attempt: a fresh consent for the right one.
    let allowed = http
        .post(format!("{base}/oauth/authorize"))
        .header("cookie", &cookie)
        .form(&form)
        .send()
        .await
        .unwrap();
    let code = param(&location(&allowed), "code").unwrap();
    let tokens: Value = http
        .post(format!("{base}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("redirect_uri", callback),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access = tokens["access_token"]
        .as_str()
        .expect("an access token")
        .to_owned();
    let refresh = tokens["refresh_token"].as_str().unwrap().to_owned();
    let replay = http
        .post(format!("{base}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status(), 400, "a code works once");
    let (status, _) = mcp(
        &base,
        &access,
        json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
    )
    .await;
    assert_eq!(status, 401, "a replayed code revokes what it bought");

    // So the person allows again, and this time the client behaves.
    let allowed = http
        .post(format!("{base}/oauth/authorize"))
        .header("cookie", &cookie)
        .form(&form)
        .send()
        .await
        .unwrap();
    let code = param(&location(&allowed), "code").unwrap();
    let tokens: Value = http
        .post(format!("{base}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("redirect_uri", callback),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access = tokens["access_token"].as_str().unwrap().to_owned();
    let refresh_now = tokens["refresh_token"].as_str().unwrap().to_owned();
    assert_ne!(refresh_now, refresh);

    // MCP: the handshake and the list, before any tab is open.
    let (status, hello) = mcp(
        &base,
        &access,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(hello["result"]["protocolVersion"], "2025-06-18");
    let (_, list) = mcp(
        &base,
        &access,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    )
    .await;
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"show_plan") && names.contains(&"create"));
    assert!(
        !names.contains(&"feedback") && !names.contains(&"run_plugin"),
        "desktop only"
    );
    let (_, page) = mcp(
        &base,
        &access,
        json!({"jsonrpc": "2.0", "id": 3, "method": "resources/read", "params": {"uri": "ui://newera/plan-viewer.html"}}),
    )
    .await;
    assert_eq!(
        page["result"]["contents"][0]["mimeType"],
        "text/html;profile=mcp-app"
    );
    // With no tab open, the calls work on a project in the cloud.
    let call = |id: u64, name: &str, arguments: Value| {
        let (base, access) = (base.clone(), access.clone());
        let name = name.to_owned();
        async move {
            let (_, answer) = mcp(
                &base,
                &access,
                json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": arguments}}),
            )
            .await;
            answer["result"].clone()
        }
    };
    let drawn = call(
        4,
        "create",
        json!({"walls": [{"pts": [[0, 0], [400, 0], [400, 300], [0, 300]], "closed": true}]}),
    )
    .await;
    assert_eq!(drawn["isError"], false, "{drawn}");
    let home = call(40, "home", json!({"detail": "summary"})).await;
    assert!(
        home["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("walls"),
        "{home}"
    );
    let listed = call(41, "projects", json!({})).await;
    let listed: Value =
        serde_json::from_str(listed["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(listed["rows"][0][0], "Projeto 1");
    let size: i32 = sqlx::query_scalar("select size from projects")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert!(size > 100, "the change was kept");

    // A photo counts, and nothing is sold: a good one is everybody's too.
    let photo = call(42, "render_photo", json!({"w": 64, "h": 48})).await;
    assert_eq!(photo["isError"], false, "{photo}");
    let used: i32 = sqlx::query_scalar("select count from usage where kind = 'draft_render'")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(used, 1);
    let good = call(
        43,
        "render_photo",
        json!({"w": 64, "h": 48, "quality": "good"}),
    )
    .await;
    assert_eq!(good["isError"], false, "{good}");

    // An export comes back as a link, and the link is the file.
    let exported = call(44, "export", json!({"path": "/etc/casa.pdf"})).await;
    let text = exported["content"][0]["text"].as_str().unwrap().to_owned();
    let link = text.split_whitespace().nth(1).expect("a link").to_owned();
    assert!(link.starts_with(&format!("{base}/files/")), "{text}");
    let file = http.get(&link).send().await.unwrap();
    assert_eq!(file.status(), 200);
    assert!(
        file.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("casa.pdf")
    );
    assert!(file.bytes().await.unwrap().starts_with(b"%PDF"));
    assert!(
        !std::path::Path::new("/etc/casa.pdf").exists(),
        "never where the path said"
    );

    // Projects by name: a new one, back to the first, renamed on save.
    let made = call(
        45,
        "file",
        json!({"action": "new", "name": "Casa da praia"}),
    )
    .await;
    assert_eq!(made["isError"], false, "{made}");
    // More projects than the free plan used to keep.
    for (id, name) in [(51, "Escritório"), (52, "Sítio"), (53, "Loja")] {
        let more = call(id, "file", json!({"action": "new", "name": name})).await;
        assert_eq!(more["isError"], false, "{more}");
    }
    let back = call(47, "file", json!({"action": "open", "path": "Projeto 1"})).await;
    assert_eq!(back["isError"], false, "{back}");
    let renamed = call(
        48,
        "file",
        json!({"action": "save", "path": "Apartamento.newera"}),
    )
    .await;
    assert_eq!(renamed["content"][0]["text"], "ok saved Apartamento");
    // A limit written back into a plan holds again, with no code: one project,
    // one draft photo a day (already taken) and no storage.
    sqlx::query(
        r#"update plans set limits = '{"projects": 1, "draft_renders_per_day": 1, "storage_mb": 0}'"#,
    )
    .execute(&state.db)
    .await
    .unwrap();
    for (id, name, arguments) in [
        (54, "file", json!({"action": "new"})),
        (55, "render_photo", json!({"w": 64, "h": 48})),
        (56, "file", json!({"action": "save"})),
    ] {
        let limited = call(id, name, arguments).await;
        assert_eq!(limited["isError"], true, "{name}: {limited}");
    }
    sqlx::query("update plans set limits = '{}'")
        .execute(&state.db)
        .await
        .unwrap();
    // "Open in the editor" is this project, which the site may fetch with the
    // person's cookie — and nobody else may.
    let shown = call(50, "show_plan", json!({})).await;
    let editor = shown["structuredContent"]["editor"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        editor.starts_with(&format!("{SITE}/editor/?project=")),
        "{editor}"
    );
    let project_url = param(
        &format!("http://x/{}", editor.split('/').next_back().unwrap()),
        "project",
    )
    .unwrap();
    let fetched = http
        .get(&project_url)
        .header("cookie", &cookie)
        .header("origin", SITE)
        .send()
        .await
        .unwrap();
    assert_eq!(fetched.status(), 200);
    assert_eq!(
        fetched.headers()["access-control-allow-credentials"],
        "true"
    );
    assert!(
        fetched.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .contains("Apartamento.newera")
    );
    assert!(fetched.bytes().await.unwrap().starts_with(b"PK"));
    let stranger = http.get(&project_url).send().await.unwrap();
    assert_ne!(stranger.status(), 200, "only its owner downloads a project");
    let refused = call(
        49,
        "edit_background",
        json!({"action": "set", "path": "/etc/passwd"}),
    )
    .await;
    assert_eq!(refused["isError"], true);

    // A tab opens a room on the relay, signs in, and claims it.
    let opened: Value = http
        .post(format!("{base}/rooms"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let room = opened["room"].as_str().unwrap().to_owned();
    let tab_key = opened["tab_key"].as_str().unwrap().to_owned();
    let tab_url = format!(
        "ws://{}{}",
        base.trim_start_matches("http://"),
        opened["tab_path"].as_str().unwrap()
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(&tab_url).await.unwrap();
    socket
        .send(tokio_tungstenite::tungstenite::Message::text(
            json!({"type": "hello", "tools": [], "resources": []}).to_string(),
        ))
        .await
        .unwrap();
    let not_from_site = http
        .post(format!("{base}/account/claim"))
        .header("cookie", &cookie)
        .header("origin", "https://evil.example")
        .json(&json!({"room": room, "tab_key": tab_key}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        not_from_site.status(),
        403,
        "only the site may claim with the cookie"
    );
    let claimed = http
        .post(format!("{base}/account/claim"))
        .header("cookie", &cookie)
        .header("origin", SITE)
        .json(&json!({"room": room, "tab_key": tab_key}))
        .send()
        .await
        .unwrap();
    assert_eq!(claimed.status(), 200);
    let me: Value = http
        .get(format!("{base}/account/me"))
        .header("cookie", &cookie)
        .header("origin", SITE)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["email"], "Ana@Example.com");
    assert_eq!(me["plan"]["code"], "free");

    // The call goes down to the tab and its answer comes back.
    let calling = tokio::spawn({
        let (base, access) = (base.clone(), access.clone());
        async move {
            mcp(
                &base,
                &access,
                json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "home", "arguments": {}}}),
            )
            .await
        }
    });
    let work = loop {
        let message = socket.next().await.unwrap().unwrap();
        let value: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        if value["type"] == "call" {
            break value;
        }
    };
    assert_eq!(work["name"], "home");
    socket
        .send(tokio_tungstenite::tungstenite::Message::text(
            json!({"type": "result", "id": work["id"], "ok": true, "result": {"content": [{"type": "text", "text": "w1"}]}}).to_string(),
        ))
        .await
        .unwrap();
    let (_, answered) = calling.await.unwrap();
    assert_eq!(answered["result"]["content"][0]["text"], "w1");

    // Refresh rotates; the old refresh token used again revokes the family.
    let rotated: Value = http
        .post(format!("{base}/oauth/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_now.as_str()),
            ("client_id", client_id.as_str()),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access2 = rotated["access_token"]
        .as_str()
        .expect("rotated")
        .to_owned();
    let (status, _) = mcp(
        &base,
        &access2,
        json!({"jsonrpc": "2.0", "id": 6, "method": "ping"}),
    )
    .await;
    assert_eq!(status, 200);
    let stolen = http
        .post(format!("{base}/oauth/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_now.as_str()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(stolen.status(), 400);
    let (status, _) = mcp(
        &base,
        &access2,
        json!({"jsonrpc": "2.0", "id": 7, "method": "ping"}),
    )
    .await;
    assert_eq!(status, 401, "reuse of a refresh token revokes its family");

    // Closing the account signs everything out.
    let page = http
        .get(format!("{base}/account"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let csrf = field(&page, "csrf");
    let closed = http
        .post(format!("{base}/account/close"))
        .header("cookie", &cookie)
        .form(&[("csrf", csrf.as_str())])
        .send()
        .await
        .unwrap();
    assert_eq!(closed.status(), 200);
    let me = http
        .get(format!("{base}/account/me"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), 401);
}

#[tokio::test]
async fn sign_in_never_redirects_elsewhere() {
    let Some((base, state)) = service().await else {
        return;
    };
    for (asked, expected) in [
        ("https://evil.example/", "/account"),
        ("//evil.example/x", "/account"),
        ("/oauth/authorize?x=1", "/oauth/authorize?x=1"),
        ("http://127.0.0.1:8790/app/", "http://127.0.0.1:8790/app/"),
    ] {
        let (_, back) = sign_in(
            &base,
            &state,
            &format!("{}@example.com", secret::id()),
            asked,
        )
        .await;
        assert_eq!(back, expected, "{asked}");
    }
}
