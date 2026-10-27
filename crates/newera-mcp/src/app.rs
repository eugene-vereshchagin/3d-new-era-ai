//! The plan, shown inside the chat.
//!
//! Chat clients that speak MCP Apps (Claude, `ChatGPT`, VS Code, …) can render a
//! small web page next to a tool's answer. `show_plan` points at one: a
//! viewer that pans and zooms the plan, asks for a 3D view and opens the
//! editor. The page is plain HTML and script with nothing to fetch — the
//! hosts sandbox it and, today, refuse WebAssembly there — so the drawing
//! arrives as SVG in the tool's structured answer and the 3D as a PNG from
//! `render_3d`, both made here.
//!
//! Every transport hands out the same page: the window and `serve` through
//! the MCP handshake, a browser tab through the relay (its `hello` carries
//! [`resources`]), the hosted service through the same handler.

use serde_json::{Value, json};

/// Where the viewer lives, as the tool's `_meta.ui.resourceUri` names it.
pub const VIEWER_URI: &str = "ui://newera/plan-viewer.html";

/// The type MCP Apps hosts look for.
pub const MIME: &str = "text/html;profile=mcp-app";

/// The tool that opens the viewer.
pub const VIEWER_TOOL: &str = "show_plan";

const VIEWER_HTML: &str = include_str!("app/plan-viewer.html");

/// The resources this server has, each with its contents, as JSON: what the
/// relay needs to answer `resources/list` and `resources/read` for a tab.
pub fn resources() -> Vec<Value> {
    let viewer = json!({
        "uri": VIEWER_URI,
        "name": "plan-viewer",
        "title": "Plan viewer",
        "description": "Interactive view of the floor plan, with a 3D view on demand.",
        "mimeType": MIME,
        "text": VIEWER_HTML,
        "_meta": meta(),
    });
    // The rules behind the reviews, the same texts the `rules` tool answers.
    let rules = crate::tools::rules::TOPICS
        .iter()
        .map(|(topic, title, text)| {
            json!({
                "uri": format!("newera://rules/{topic}"),
                "name": format!("rules-{topic}"),
                "title": title,
                "mimeType": "text/markdown",
                "text": text,
            })
        });
    std::iter::once(viewer).chain(rules).collect()
}

/// What `resources/list` answers: the resources without their contents.
pub fn resource_list() -> Value {
    let listed: Vec<Value> = resources()
        .into_iter()
        .map(|mut r| {
            if let Some(map) = r.as_object_mut() {
                map.remove("text");
            }
            r
        })
        .collect();
    json!({ "resources": listed })
}

/// What `resources/read` answers for `uri`, or `None` for one nobody has.
pub fn read(uri: &str) -> Option<Value> {
    resources()
        .into_iter()
        .find(|r| r["uri"] == uri)
        .map(|r| json!({ "contents": [r] }))
}

/// The page asks for nothing from the network: no domains to allow. It
/// prefers a border, being a drawing on white.
fn meta() -> Value {
    json!({ "ui": { "csp": { "connectDomains": [], "resourceDomains": [] }, "prefersBorder": true } })
}

/// The extension a client names at `initialize` when it can display the
/// viewer.
pub const EXTENSION: &str = "io.modelcontextprotocol/ui";

/// Whether a client that said `capabilities` at `initialize` displays the
/// viewer. One that does not gets no drawing: a CLI would hand the whole SVG
/// to the model instead.
pub(crate) fn shows_apps(capabilities: &rmcp::model::ClientCapabilities) -> bool {
    capabilities
        .extensions
        .as_ref()
        .is_some_and(|e| e.contains_key(EXTENSION))
        || capabilities
            .experimental
            .as_ref()
            .is_some_and(|e| e.contains_key(EXTENSION))
}

/// Links `show_plan` to the viewer. Every other tool is left alone, and
/// stays callable by the viewer as well as the model, which is the default.
pub(crate) fn link(tool: &mut rmcp::model::Tool) {
    if tool.name != VIEWER_TOOL {
        return;
    }
    let mut meta = tool.meta.take().unwrap_or_default();
    meta.insert(
        "ui".to_owned(),
        json!({ "resourceUri": VIEWER_URI, "visibility": ["model", "app"] }),
    );
    // The older, flat key some hosts still read.
    meta.insert("ui/resourceUri".to_owned(), json!(VIEWER_URI));
    tool.meta = Some(meta);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_viewer_tool_points_at_the_viewer() {
        let tool = crate::tools()
            .into_iter()
            .find(|t| t.name == VIEWER_TOOL)
            .expect("show_plan exists");
        let meta = serde_json::to_value(tool.meta.expect("meta")).unwrap();
        assert_eq!(meta["ui"]["resourceUri"], VIEWER_URI);
        assert!(read(VIEWER_URI).is_some());
    }

    #[test]
    fn the_viewer_is_a_self_contained_page() {
        let page = &read(VIEWER_URI).unwrap()["contents"][0];
        assert_eq!(page["mimeType"], MIME);
        let html = page["text"].as_str().unwrap();
        assert!(html.contains("ui/initialize"));
        // Nothing to fetch: the host's sandbox would refuse it anyway.
        for outside in ["<script src", "<link ", "http://", "fetch("] {
            assert!(!html.contains(outside), "the page reaches out: {outside}");
        }
    }

    #[test]
    fn an_empty_plan_has_no_negative_area() {
        let document = newera_core::SharedDocument::new(newera_core::Document::default());
        let shown = crate::call(document, VIEWER_TOOL, serde_json::json!({})).unwrap();
        let text = shown.content[0].as_text().unwrap().text.clone();
        assert!(text.contains(" 0 m²") && !text.contains("-0"), "{text}");
    }

    #[test]
    fn only_a_client_with_a_viewer_gets_the_drawing() {
        let caps = |v: serde_json::Value| -> rmcp::model::ClientCapabilities {
            serde_json::from_value(v).unwrap()
        };
        assert!(shows_apps(&caps(json!({"extensions": {
            "io.modelcontextprotocol/ui": {"mimeTypes": [MIME]}
        }}))));
        assert!(!shows_apps(&caps(json!({}))));
        assert!(!shows_apps(&caps(json!({"roots": {}}))));
        let document = newera_core::SharedDocument::new(newera_core::Document::default());
        let server = crate::NewEraMcp::new(document);
        let viewer = server.shown_plan(true);
        assert!(viewer.structured_content.unwrap()["svg"].is_string());
        // Without a viewer: the summary and the editor link, and no SVG.
        let cli = server.shown_plan(false);
        let fields = cli.structured_content.unwrap();
        assert!(fields.get("svg").is_none(), "{fields}");
        assert!(fields["editor"].is_string() && fields["summary"].is_object());
        let text = cli.content[0].as_text().unwrap().text.clone();
        assert!(text.contains("no viewer"), "{text}");
    }

    #[test]
    fn listing_leaves_the_contents_out() {
        let list = resource_list();
        assert_eq!(list["resources"][0]["uri"], VIEWER_URI);
        assert!(list["resources"][0].get("text").is_none());
        assert!(read("ui://nothing").is_none());
    }
}
