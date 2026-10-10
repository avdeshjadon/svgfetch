//! Remote curation: trigger GitHub Actions `repository_dispatch` to queue
//! a brand for community curation in `avdeshjadon/svgfetch-icons`.
//!
//! Requires the env var `SVGFETCH_GITHUB_TOKEN` (a PAT with `repo` scope or
//! a fine-grained token with "Contents: write" on `svgfetch-icons`).
//!
//! The dispatch event type is `"curate-brand"` and the payload is:
//!   { "brand": "<name>", "category": "<category>" }

use serde_json::json;

const ICONS_REPO: &str = "avdeshjadon/svgfetch-icons";
const DISPATCH_EVENT: &str = "curate-brand";

/// Result of a remote curation request.
#[derive(Debug, PartialEq, Eq)]
pub enum RemoteCurationResult {
    /// Dispatch accepted — GH Actions will handle the rest.
    Queued,
    /// No token configured; user should set SVGFETCH_GITHUB_TOKEN.
    NoToken,
    /// GitHub API returned an unexpected status.
    ApiError(u16),
    /// Network / serialization error.
    NetworkError(String),
}

/// Fire a `repository_dispatch` event on `svgfetch-icons` asking the CI to
/// fetch and curate the given brand.
///
/// This is a best-effort, non-blocking helper. Callers should print a friendly
/// hint to the user regardless of the return value.
pub async fn trigger_remote_curation(brand_name: &str, category: &str) -> RemoteCurationResult {
    let token = match std::env::var("SVGFETCH_GITHUB_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => return RemoteCurationResult::NoToken,
    };

    let url = format!("https://api.github.com/repos/{}/dispatches", ICONS_REPO);

    let body = json!({
        "event_type": DISPATCH_EVENT,
        "client_payload": {
            "brand": brand_name,
            "category": category,
            "requester": "svgfetch-cli"
        }
    });

    let client = match reqwest::Client::builder()
        .user_agent(concat!("svgfetch/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => return RemoteCurationResult::NetworkError(e.to_string()),
    };

    match client
        .post(&url)
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            // 204 No Content = accepted by GitHub
            if status == 204 {
                RemoteCurationResult::Queued
            } else {
                RemoteCurationResult::ApiError(status)
            }
        }
        Err(e) => RemoteCurationResult::NetworkError(e.to_string()),
    }
}

/// Pretty-print the result of a remote curation request to stderr.
pub fn report_remote_curation(brand_name: &str, result: &RemoteCurationResult) {
    match result {
        RemoteCurationResult::Queued => {
            eprintln!(
                "[sync] Remote curation queued for \"{}\". The svgfetch-icons CI will process it shortly.",
                brand_name
            );
            eprintln!(
                "[info] Track progress at: https://github.com/{}/actions",
                ICONS_REPO
            );
        }
        RemoteCurationResult::NoToken => {
            eprintln!("[warn] Set SVGFETCH_GITHUB_TOKEN to enable automatic remote curation.");
            eprintln!(
                "[info] Create a token: https://github.com/settings/tokens (needs repo scope on svgfetch-icons)"
            );
            eprintln!(
                "[info] Or request manually: https://github.com/{}/issues",
                ICONS_REPO
            );
        }
        RemoteCurationResult::ApiError(code) => {
            eprintln!(
                "[error] GitHub API returned HTTP {} for remote curation of \"{}\".",
                code, brand_name
            );
            if *code == 401 || *code == 403 {
                eprintln!(
                    "[warn] Check that SVGFETCH_GITHUB_TOKEN has `repo` scope on `svgfetch-icons`."
                );
            }
        }
        RemoteCurationResult::NetworkError(msg) => {
            eprintln!(
                "[error] Network error while queuing remote curation: {}",
                msg
            );
        }
    }
}
