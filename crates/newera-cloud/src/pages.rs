//! The few pages a person sees here: sign in, check your email, allow an
//! AI client, and the account. Plain HTML, no script, in the site's paper
//! and ink, in Portuguese or English by the browser's language.

use axum::http::HeaderMap;
use axum::response::Html;

/// The two languages the pages speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Pt,
    En,
}

impl Lang {
    /// Portuguese for a browser that puts it first, English otherwise.
    pub fn of(headers: &HeaderMap) -> Self {
        let wanted = headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if wanted.trim_start().to_ascii_lowercase().starts_with("pt") {
            Self::Pt
        } else {
            Self::En
        }
    }

    /// This language's words.
    pub fn pick<'a>(self, pt: &'a str, en: &'a str) -> &'a str {
        match self {
            Self::Pt => pt,
            Self::En => en,
        }
    }
}

/// Text made safe to put inside HTML, attributes included.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// A whole page around `body`, which must already be escaped.
pub fn page(lang: Lang, title: &str, body: &str) -> Html<String> {
    let html_lang = lang.pick("pt-BR", "en");
    Html(format!(
        r#"<!doctype html>
<html lang="{html_lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="referrer" content="no-referrer">
<title>{title} — 3D New Era AI</title>
<style>{STYLE}</style>
</head>
<body>
<main class="card">
<div class="top"><p class="brand">3D New Era <em>AI</em></p>{coffee}</div>
{body}
</main>
<p class="foot"><a href="https://3dneweraai.com/privacy/">{privacy}</a> · <a href="https://3dneweraai.com/terms/">{terms}</a> · <a href="https://3dneweraai.com/refund/">{refund}</a></p>
</body>
</html>"#,
        title = escape(title),
        coffee = coffee(lang),
        privacy = lang.pick("Privacidade", "Privacy"),
        terms = lang.pick("Termos", "Terms"),
        refund = lang.pick("Reembolso", "Refunds"),
    ))
}

/// Where the support button goes: a donation, which buys nothing.
const SUPPORT_URL: &str = "https://buymeacoffee.com/leandrodaf";

/// Buy Me a Coffee's button, on every page here.
///
/// The site draws it in Buy Me a Coffee's own script face; these pages ask
/// for no font, no script and no third-party anything — a page someone is
/// signed into should not start making requests elsewhere — so it keeps the
/// yellow and the cup and sets the words in the face the page already uses.
fn coffee(lang: Lang) -> String {
    let cup = r#"<svg viewBox="0 0 26 32" aria-hidden="true"><rect x="2.6" y="1.6" width="20.8" height="5.6" rx="2.6"/><path d="M4.9 10.2h16.2l-1.9 17.3a2.7 2.7 0 0 1-2.7 2.4H9.5a2.7 2.7 0 0 1-2.7-2.4z"/></svg>"#;
    format!(
        r#"<a class="bmc" href="{SUPPORT_URL}" target="_blank" rel="noopener">{cup}<span>{label}</span></a>"#,
        label = lang.pick("Me paga um café", "Buy me a coffee"),
    )
}

const STYLE: &str = "
:root{--paper:#f4f1ea;--card:#fbfaf6;--ink:#161513;--ink2:#3a3833;--graphite:#6d6a62;--rule:#d9d3c5;--accent:#2446d8;color-scheme:light}
@media (prefers-color-scheme:dark){:root{--paper:#12110f;--card:#1c1b18;--ink:#f1ede4;--ink2:#d4cfc4;--graphite:#9a958a;--rule:#2e2c27;--accent:#7d93ff;color-scheme:dark}}
*{box-sizing:border-box}
body{margin:0;min-height:100vh;display:flex;flex-direction:column;align-items:center;justify-content:center;background:var(--paper);color:var(--ink2);font:16px/1.55 ui-sans-serif,system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;padding:24px 16px}
.card{width:100%;max-width:420px;background:var(--card);border:1px solid var(--rule);border-radius:18px;padding:28px 24px}
.brand{margin:0 0 18px;font-weight:700;color:var(--ink)}.brand em{font-style:normal;color:var(--accent)}
h1{margin:0 0 8px;font-size:1.35rem;line-height:1.25;color:var(--ink)}
p{margin:0 0 14px}.muted{color:var(--graphite);font-size:.92rem}
label{display:block;font-size:.9rem;margin:0 0 6px;color:var(--graphite)}
input[type=email]{width:100%;font:inherit;padding:11px 12px;border:1px solid var(--rule);border-radius:10px;background:var(--paper);color:var(--ink)}
button,.button{display:inline-flex;align-items:center;justify-content:center;width:100%;margin-top:12px;font:inherit;font-weight:600;padding:11px 16px;border-radius:10px;border:1px solid var(--ink);background:var(--ink);color:var(--paper);cursor:pointer;text-decoration:none}
.ghost{background:transparent;color:var(--ink);border-color:var(--rule)}
.row{display:flex;gap:10px}.row>*{flex:1}
code{font:.9em ui-monospace,Menlo,Consolas,monospace;overflow-wrap:anywhere}
.foot{margin-top:18px;font-size:.85rem;color:var(--graphite)}a{color:var(--accent)}
.top{display:flex;align-items:center;justify-content:space-between;gap:12px;margin:0 0 18px}
.top .brand{margin:0}
.bmc{display:inline-flex;align-items:center;gap:7px;flex:none;padding:6px 11px 7px;border-radius:9px;background:#ffdd00;color:#0d0c0c;font-size:.88rem;font-weight:600;text-decoration:none;box-shadow:0 1px 0 rgba(0,0,0,.16)}
.bmc svg{width:14px;height:17px;flex:none;fill:#fff;stroke:#0d0c0c;stroke-width:2.1;stroke-linejoin:round}
@media (max-width:360px){.bmc span{position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)}}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_cannot_open_a_tag() {
        assert_eq!(
            escape(r#"<script>"x" & 'y'</script>"#),
            "&lt;script&gt;&quot;x&quot; &amp; &#39;y&#39;&lt;/script&gt;"
        );
    }

    #[test]
    fn portuguese_for_a_portuguese_browser() {
        let mut headers = HeaderMap::new();
        assert_eq!(Lang::of(&headers), Lang::En);
        headers.insert(
            "accept-language",
            "pt-BR,pt;q=0.9,en;q=0.8".parse().unwrap(),
        );
        assert_eq!(Lang::of(&headers), Lang::Pt);
    }
}
