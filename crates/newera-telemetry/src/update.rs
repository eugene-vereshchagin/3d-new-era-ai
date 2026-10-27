//! Whether a newer release is out, so the window can say so.
//!
//! One request at launch to GitHub's public API for the repository's latest
//! release (prereleases left out), compared with this build's version. Nothing
//! about the installation goes with it; GitHub sees the address it comes from,
//! as for any download. No answer, or a slow one, is the same as no news.

use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/leandrodaf/3d-new-era-ai/releases/latest";

/// The newer version, `3.2.0` style, when the latest release is ahead of this
/// build. Blocks for up to a few seconds: call it off the window's thread.
pub fn newer_release() -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();
    let release: serde_json::Value = agent
        .get(LATEST)
        .header("accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_json()
        .ok()?;
    let latest = release["tag_name"].as_str()?.trim_start_matches('v');
    is_newer(latest, env!("CARGO_PKG_VERSION")).then(|| latest.to_owned())
}

/// `major.minor.patch` of a version, `None` for anything else.
fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    Some((next()?, next()?, next()?))
}

/// Whether `latest` is a later release than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse(latest), parse(current)), (Some(l), Some(c)) if l > c)
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn a_later_release_is_news_and_nothing_else_is() {
        assert!(is_newer("3.2.0", "3.1.0"));
        assert!(is_newer("v3.10.0", "3.9.9"), "numbers, not text");
        assert!(is_newer("4.0.0", "3.99.99"));
        assert!(!is_newer("3.1.0", "3.1.0"));
        assert!(!is_newer("3.0.9", "3.1.0"));
        assert!(!is_newer("nightly", "3.1.0"));
    }
}
