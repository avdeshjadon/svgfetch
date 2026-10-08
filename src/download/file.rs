//! Single-file download: streaming to a temp file, then atomic rename.

use std::path::{Path, PathBuf};

use futures::StreamExt;
use reqwest::Client;
use tokio_util::sync::CancellationToken;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::security;

/// Upper bound for a single downloaded file (default 100 MiB) — an SVG larger than
/// this is almost certainly a mislabeled response; refuse rather than fill
/// the user's disk.
pub const DEFAULT_MAX_FILE_BYTES: u64 = crate::config::DEFAULT_MAX_DOWNLOAD_MB * 1024 * 1024;

/// A pluggable progress callback, invoked with (bytes written, total bytes).
/// The total is 0 while it is unknown. Must be callable from spawned tasks.
pub type ProgressFn = Box<dyn FnMut(u64, u64) + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadOutcome {
    Written {
        path: PathBuf,
        bytes: u64,
        sha256: String,
    },
    Skipped,
}

/// Validate whether byte slice represents legitimate SVG content.
/// Rejects HTML error pages, truncated/empty files, binary non-SVG data, etc.
pub fn validate_svg_content(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Err(Error::Download("downloaded file is empty (0 bytes)".into()));
    }
    if bytes.len() < 8 {
        return Err(Error::Download(
            "downloaded file is too small to be a valid SVG".into(),
        ));
    }

    // Check for null bytes in initial header (indicates binary file like PNG, EXE, etc.)
    let probe_len = bytes.len().min(1024);
    if bytes[..probe_len].contains(&0) {
        return Err(Error::Download(
            "downloaded content contains binary null bytes and is not an SVG".into(),
        ));
    }

    // Validate UTF-8 text representation
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::Download(
                "downloaded file is not valid UTF-8 SVG XML text".into(),
            ));
        }
    };

    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();

    // Check if it's an HTML error/maintenance page
    if (lower.starts_with("<!doctype html") || lower.starts_with("<html"))
        && !lower.contains("<svg")
    {
        return Err(Error::Download(
            "downloaded content is an HTML document, not an SVG".into(),
        ));
    }

    // Must contain <svg element
    if !lower.contains("<svg") {
        return Err(Error::Download(
            "downloaded content does not contain an <svg> element".into(),
        ));
    }

    // Must contain </svg> or self-closing />
    if !lower.contains("</svg>") && !lower.contains("/>") {
        return Err(Error::Download(
            "downloaded SVG is truncated (missing closing tag)".into(),
        ));
    }

    Ok(())
}

/// Allocates collision-free file names inside a directory.
///
/// Handles: existing files, case-insensitive collisions, and names already
/// claimed by other files in the same batch.
#[derive(Debug, Default)]
pub struct NameAllocator {
    used: std::collections::HashSet<String>,
}

impl NameAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve `name` in `dest`. Returns the full path to write to, or
    /// `None` when the file exists and overwriting is disabled.
    pub fn allocate(&mut self, dest: &Path, name: &str, overwrite: bool) -> Option<PathBuf> {
        let base = security::sanitize_filename(name);
        let (stem, ext) = split_ext(&base);

        for attempt in 0..10_000u32 {
            let candidate = if attempt == 0 {
                base.clone()
            } else {
                format!("{stem}-{attempt}{ext}")
            };
            let key = security::casefold(&candidate);
            if self.used.contains(&key) {
                continue;
            }
            let path = match security::safe_join(dest, &candidate) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if path.exists() && !overwrite {
                // Claim it so a later duplicate does not loop over it too.
                self.used.insert(key);
                return None;
            }
            self.used.insert(key);
            return Some(path);
        }
        None
    }
}

fn split_ext(name: &str) -> (String, String) {
    match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// Resolve a unique, safe path for `name` in `dir` without allocating.
pub fn unique_path(dir: &Path, name: &str, overwrite: bool) -> Option<PathBuf> {
    NameAllocator::new().allocate(dir, name, overwrite)
}

/// Stream a URL to `path` with default size limit.
pub async fn download_one(
    client: &Client,
    url: &str,
    path: &Path,
    cancel: &CancellationToken,
    progress: Option<ProgressFn>,
) -> Result<DownloadOutcome> {
    download_one_with_limit(client, url, path, DEFAULT_MAX_FILE_BYTES, cancel, progress).await
}

/// Stream a URL to `path` with configurable size limit.
///
/// * writes to `<path>.part-<pid>` first, validates SVG XML, then renames (atomic on POSIX),
/// * computes SHA-256 checksum during streaming,
/// * honors cancellation at every chunk,
/// * enforces a configurable size ceiling and content-type sanity check,
/// * reports progress through `progress` when a total size is known.
pub async fn download_one_with_limit(
    client: &Client,
    url: &str,
    path: &Path,
    max_file_bytes: u64,
    cancel: &CancellationToken,
    mut progress: Option<ProgressFn>,
) -> Result<DownloadOutcome> {
    #[cfg(test)]
    let is_valid_url = security::validate_https_url(url)
        || url.starts_with("http://127.0.0.1")
        || url.starts_with("http://localhost");
    #[cfg(not(test))]
    let is_valid_url = security::validate_https_url(url);

    if !is_valid_url {
        return Err(Error::InvalidUrl(url.to_string()));
    }
    if path.exists() {
        return Ok(DownloadOutcome::Skipped);
    }

    const MAX_ATTEMPTS: u32 = 5;
    let mut last_error: Option<Error> = None;

    for attempt in 0..MAX_ATTEMPTS {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }

        // Apply backoff if this is a retry attempt
        if attempt > 0 {
            let backoff_duration = match &last_error {
                Some(Error::Http {
                    status: 429,
                    detail,
                }) => crate::api::rate_limit::parse_retry_after(detail)
                    .unwrap_or_else(|| crate::api::rate_limit::backoff(attempt))
                    .min(std::time::Duration::from_secs(10)),
                _ => {
                    crate::api::rate_limit::backoff(attempt).min(std::time::Duration::from_secs(8))
                }
            };

            tracing::debug!(
                "Retrying download of {} in {:?} (attempt {}/{MAX_ATTEMPTS})",
                url,
                backoff_duration,
                attempt + 1
            );

            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                _ = tokio::time::sleep(backoff_duration) => {}
            }
        }

        let request = client
            .get(url)
            .header(reqwest::header::ACCEPT, "image/svg+xml,image/*,*/*;q=0.8")
            .header(reqwest::header::REFERER, "https://commons.wikimedia.org/")
            .header("Sec-Fetch-Dest", "image")
            .header("Sec-Fetch-Mode", "no-cors")
            .header("Sec-Fetch-Site", "cross-site")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9");

        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            res = request.send() => match res {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(Error::from(e));
                    continue;
                }
            },
        };

        let status = response.status();
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            last_error = Some(Error::Http {
                status: 429,
                detail: retry_after,
            });
            continue;
        }

        if status.is_server_error() {
            last_error = Some(Error::Http {
                status: status.as_u16(),
                detail: format!("server error: {status}"),
            });
            continue;
        }

        if !status.is_success() {
            // Client errors (404, 400, etc.) are non-retryable
            return Err(Error::Http {
                status: status.as_u16(),
                detail: "download request failed".to_string(),
            });
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Wikimedia serves SVGs as image/svg+xml or octet-stream; an HTML body
        // here means we landed on a transient CDN error/throttling notice — retry.
        if content_type.contains("text/html") {
            last_error = Some(Error::Download(format!(
                "unexpected HTML content type from server: {content_type}"
            )));
            continue;
        }

        let total = response.content_length().unwrap_or(0);
        if total > max_file_bytes {
            return Err(Error::Download(format!(
                "file is larger than the {max_file_bytes} byte limit"
            )));
        }

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;

        let tmp = temp_path_for(path);
        let write =
            write_stream(response, &tmp, total, max_file_bytes, cancel, &mut progress).await;

        match write {
            Ok(payload) => {
                // Read and validate SVG structure before atomic rename
                let read_res = std::fs::read(&tmp);
                match read_res {
                    Ok(bytes) => {
                        if let Err(val_err) = validate_svg_content(&bytes) {
                            let _ = std::fs::remove_file(&tmp);
                            return Err(val_err);
                        }
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&tmp);
                        return Err(Error::Io(e));
                    }
                }

                // Atomic publish: only fully-written and validated files get their final name.
                match std::fs::rename(&tmp, path) {
                    Ok(()) => {
                        return Ok(DownloadOutcome::Written {
                            path: path.to_path_buf(),
                            bytes: payload.bytes,
                            sha256: payload.sha256,
                        })
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&tmp);
                        return Err(Error::Io(e));
                    }
                }
            }
            Err(err) => {
                let _ = std::fs::remove_file(&tmp);
                if matches!(err, Error::Cancelled) {
                    return Err(Error::Cancelled);
                }
                last_error = Some(err);
                continue;
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| Error::Download("download failed after maximum retry attempts".into())))
}

fn temp_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    name.push(format!(".part-{}", std::process::id()));
    path.with_file_name(name)
}

struct StreamPayload {
    bytes: u64,
    sha256: String,
}

async fn write_stream(
    response: reqwest::Response,
    tmp: &Path,
    total: u64,
    max_bytes: u64,
    cancel: &CancellationToken,
    progress: &mut Option<ProgressFn>,
) -> Result<StreamPayload> {
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::fs::File::create(tmp).await?;
    let mut written: u64 = 0;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            next = stream.next() => match next {
                None => break,
                Some(Err(e)) => return Err(Error::from(e)),
                Some(Ok(bytes)) => bytes,
            }
        };
        written += chunk.len() as u64;
        if written > max_bytes {
            return Err(Error::Download(
                "file exceeded size limit mid-download".into(),
            ));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        if let Some(cb) = progress.as_mut() {
            cb(written, total);
        }
    }

    file.flush().await?;
    file.sync_all().await?;
    let sha256 = format!("{:x}", hasher.finalize());
    Ok(StreamPayload {
        bytes: written,
        sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_handles_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = NameAllocator::new();
        let p1 = a.allocate(dir.path(), "GitHub.svg", false).unwrap();
        assert!(p1.ends_with("GitHub.svg"));
        let p2 = a.allocate(dir.path(), "GitHub.svg", false).unwrap();
        assert!(p2.ends_with("GitHub-1.svg"), "{p2:?}");
        let p3 = a.allocate(dir.path(), "GitHub.svg", false).unwrap();
        assert!(p3.ends_with("GitHub-2.svg"), "{p3:?}");
    }

    #[test]
    fn allocator_skips_existing_without_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("exists.svg"), "x").unwrap();
        let mut a = NameAllocator::new();
        assert!(a.allocate(dir.path(), "exists.svg", false).is_none());
        // With overwrite, the original name is returned.
        let mut a = NameAllocator::new();
        assert!(a.allocate(dir.path(), "exists.svg", true).is_some());
    }

    #[test]
    fn allocator_is_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = NameAllocator::new();
        let _ = a.allocate(dir.path(), "GitHub.svg", false);
        let p = a.allocate(dir.path(), "github.svg", false).unwrap();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        assert_ne!(
            name.to_lowercase(),
            "github.svg",
            "casefold collision not handled: {name}"
        );
    }

    #[test]
    fn allocator_survives_traversal_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = NameAllocator::new();
        let p = a.allocate(dir.path(), "../../evil.svg", false).unwrap();
        assert!(p.starts_with(dir.path()));
        assert!(!p.to_string_lossy().contains(".."));
    }

    #[test]
    fn rejects_non_https_urls() {
        assert!(!security::validate_https_url("http://x.test/a.svg"));
        assert!(!security::validate_https_url("ftp://x.test/a.svg"));
    }

    #[tokio::test]
    async fn download_one_retries_on_429_and_succeeds() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            // First request: 429 Too Many Requests with Retry-After: 0
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let response = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            }

            // Second request: 200 OK with valid SVG
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let body = "<svg><circle r='10'/></svg>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            }
        });

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let out_path = dir.path().join("icon.svg");
        let cancel = CancellationToken::new();
        let server_url = format!("http://127.0.0.1:{port}/icon.svg");

        let res = download_one(&client, &server_url, &out_path, &cancel, None).await;
        assert!(res.is_ok(), "download should succeed on retry: {:?}", res);
        assert!(out_path.exists(), "target file should be written");
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert_eq!(content, "<svg><circle r='10'/></svg>");
        match res.unwrap() {
            DownloadOutcome::Written { sha256, bytes, .. } => {
                assert_eq!(bytes, content.len() as u64);
                assert!(!sha256.is_empty());
            }
            DownloadOutcome::Skipped => panic!("expected written outcome"),
        }
    }

    #[test]
    fn validate_svg_content_accepts_valid_svgs() {
        assert!(validate_svg_content(b"<svg viewBox='0 0 10 10'><circle r='5'/></svg>").is_ok());
        assert!(validate_svg_content(b"<?xml version='1.0'?><svg xmlns='http://www.w3.org/2000/svg'><path d='M0 0h10v10H0z'/></svg>").is_ok());
        assert!(
            validate_svg_content(b"<!-- comment --><svg><rect width='10' height='10'/></svg>")
                .is_ok()
        );
        assert!(validate_svg_content(b"<svg width='10' height='10'/>").is_ok());
    }

    #[test]
    fn validate_svg_content_rejects_invalid_content() {
        // Empty
        assert!(validate_svg_content(b"").is_err());
        // Too short
        assert!(validate_svg_content(b"<svg>").is_err());
        // HTML error page
        assert!(
            validate_svg_content(b"<!DOCTYPE html><html><body>Error 404</body></html>").is_err()
        );
        // Binary with null bytes
        assert!(validate_svg_content(&[0x89, 0x50, 0x4E, 0x47, 0x00, 0x0D, 0x0A, 0x1A]).is_err());
        // Truncated (no closing tag)
        assert!(validate_svg_content(b"<svg><path d='M 0 0").is_err());
    }
}
