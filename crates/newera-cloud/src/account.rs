//! The account: its page (projects, sign out, close), and what the editor in a
//! browser tab asks of it — who is signed in, and "this tab is mine".

use axum::Json;
use axum::extract::{Form, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use crate::login::{self, SESSION};
use crate::pages::{Lang, escape, page};
use crate::{AppState, accounts, secret};

fn close_token(session: &str) -> String {
    secret::hash(&format!("close:{session}"))
}

/// The account page.
pub async fn show(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let lang = Lang::of(&headers);
    let Some(account) = login::signed_in(&app, &headers).await else {
        return Redirect::to("/login?return_to=/account").into_response();
    };
    let session = login::cookie(&headers, SESSION).unwrap_or_default();
    let body = format!(
        r#"<h1>{title}</h1>
<p>{email}</p>
{projects}
<a class="button" href="https://3dneweraai.com/app/">{editor}</a>
<a class="button ghost" href="/logout">{out}</a>
<details><summary class="muted">{close}</summary>
<form method="post" action="/account/close">
<p class="muted">{close_text}</p>
<input type="hidden" name="csrf" value="{csrf}">
<button class="ghost" type="submit">{close_button}</button>
</form>
</details>"#,
        title = lang.pick("Sua conta", "Your account"),
        email = escape(&account.email),
        projects = project_list(&app, &account.id, lang).await,
        editor = lang.pick("Abrir o editor", "Open the editor"),
        out = lang.pick("Sair", "Sign out"),
        close = lang.pick("Apagar a conta", "Close the account"),
        close_text = lang.pick(
            "Encerra o acesso na hora, desconecta as IAs ligadas a ela e apaga tudo em até 30 dias.",
            "Ends access at once, disconnects the AIs linked to it and deletes everything within 30 days."
        ),
        csrf = close_token(&session),
        close_button = lang.pick("Apagar minha conta", "Close my account"),
    );
    page(lang, lang.pick("Sua conta", "Your account"), &body).into_response()
}

/// The account's cloud projects, each with its download.
async fn project_list(app: &AppState, account: &str, lang: Lang) -> String {
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        "select id, name, size from projects where account_id = $1 order by updated_at desc",
    )
    .bind(account)
    .fetch_all(&app.db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return String::new();
    }
    let items = rows.iter().fold(String::new(), |mut out, (id, name, size)| {
        use std::fmt::Write as _;
        let _ = write!(
            out,
            r#"<li><a href="/account/projects/{id}">{name}.newera</a> <span class="muted">{kb} KB</span></li>"#,
            id = escape(id),
            name = escape(name),
            kb = size / 1024,
        );
        out
    });
    format!(
        r#"<p class="muted">{title}</p><ul>{items}</ul>"#,
        title = lang.pick(
            "Projetos na nuvem (baixe e abra no editor ou no app):",
            "Projects in the cloud (download, and open in the editor or the app):"
        ),
    )
}

/// A cloud project as a `.newera` file, for its owner.
pub async fn download(
    State(app): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let Some(account) = login::signed_in(&app, &headers).await else {
        return Redirect::to("/login?return_to=/account").into_response();
    };
    let found: Option<(String, Vec<u8>)> =
        sqlx::query_as("select name, data from projects where id = $1 and account_id = $2")
            .bind(&id)
            .bind(&account.id)
            .fetch_optional(&app.db)
            .await
            .ok()
            .flatten();
    match found {
        Some((name, data)) => {
            crate::files::download(&format!("{name}.newera"), "application/zip", data)
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CloseForm {
    csrf: String,
}

/// Where signing in from the editor ends: the editor's own tab picks the
/// account up by itself, so this one only says so.
pub async fn done(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let lang = Lang::of(&headers);
    let Some(account) = login::signed_in(&app, &headers).await else {
        return Redirect::to("/login?return_to=/account/done").into_response();
    };
    let body = format!(
        "<h1>{}</h1><p>{} <b>{}</b>.</p><p class=\"muted\">{}</p>",
        lang.pick("Pronto", "All set"),
        lang.pick("Você entrou como", "You are signed in as"),
        escape(&account.email),
        lang.pick(
            "Pode fechar esta aba e voltar para o editor: ele se conecta à sua conta sozinho.",
            "You can close this tab and go back to the editor: it links to your account by itself."
        ),
    );
    page(lang, lang.pick("Pronto", "All set"), &body).into_response()
}

/// Closes the account.
pub async fn close(
    State(app): State<AppState>,
    headers: HeaderMap,
    Form(f): Form<CloseForm>,
) -> Response {
    let lang = Lang::of(&headers);
    let Some(account) = login::signed_in(&app, &headers).await else {
        return Redirect::to("/login").into_response();
    };
    let session = login::cookie(&headers, SESSION).unwrap_or_default();
    if !secret::same(&f.csrf, &close_token(&session)) {
        return (StatusCode::BAD_REQUEST, "the form expired").into_response();
    }
    if let Err(err) = accounts::close(&app.db, &account.id).await {
        tracing::error!("closing an account: {err}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let body = format!(
        "<h1>{}</h1><p class=\"muted\">{}</p>",
        lang.pick("Conta apagada", "Account closed"),
        lang.pick(
            "Tudo o que era dela some em até 30 dias.",
            "Everything that was in it goes within 30 days."
        )
    );
    let mut response = page(lang, "Account", &body).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        header::HeaderValue::from_static(
            "newera_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        ),
    );
    response
}

/// A request from the site with the person's cookie must come from the
/// site: the browser sets `Origin` and a page cannot forge it.
fn from_site(app: &AppState, headers: &HeaderMap) -> bool {
    headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|origin| app.config.site_origins.iter().any(|o| o == origin))
}

/// Who is signed in, for the editor's AI panel.
pub async fn me(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let Some(account) = login::signed_in(&app, &headers).await else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"signed_in": false}))).into_response();
    };
    let plan = accounts::plan_of(&app.db, &account.id).await.ok();
    Json(json!({
        "signed_in": true,
        "email": account.email,
        "plan": plan,
        "mcp_url": format!("{}/mcp", app.config.public_url),
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct Claim {
    room: String,
    tab_key: String,
}

/// "This tab is mine": the editor, signed in, hands its relay room to the
/// account, so the account's AI clients reach it at the fixed address.
pub async fn claim(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(c): Json<Claim>,
) -> Response {
    if !from_site(&app, &headers) {
        return (StatusCode::FORBIDDEN, "only the editor can claim a room").into_response();
    }
    let Some(account) = login::signed_in(&app, &headers).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match app.rooms.claim(&c.room, &c.tab_key, &account.id) {
        Ok(()) => Json(json!({"ok": true, "mcp_url": format!("{}/mcp", app.config.public_url)}))
            .into_response(),
        Err(why) => (StatusCode::FORBIDDEN, why).into_response(),
    }
}
