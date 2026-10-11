//! Self-update: fetch the latest GitHub release and replace the running binary.
//!
//! `svgfetch update` resolves the newest tagged release for the current
//! platform, downloads the packaged archive, verifies its SHA-256, and swaps
//! the installed `svgfetch` binaries in place. Older binaries and any
//! leftover `.old` files are removed so no stale copies survive.

use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};
use std::time::Instant;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::ui::{self, LogLine, Progress, Ui, MIN_STEP};
use crate::error::{Error, Result};
use crate::models::format_size;

/// Base of the GitHub API URL. Overridable for testing via
/// `SVGFETCH_UPDATE_REPO` (must point at an API root that serves
/// `/repos/avdeshjadon/svgfetch/releases/latest`).
fn api_root() -> String {
    std::env::var("SVGFETCH_UPDATE_REPO")
        .or_else(|_| std::env::var("GET_SVG_UPDATE_REPO"))
        .unwrap_or_else(|_| "https://api.github.com/repos/avdeshjadon/svgfetch".to_string())
}

fn repo_home() -> &'static str {
    "https://github.com/avdeshjadon/svgfetch"
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// Map the running OS/CPU to the artifact triple used by the release workflow.
fn target_triple() -> Result<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let triple = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => {
            return Err(Error::Other(format!(
                "no prebuilt binary exists for your system ({os}/{arch}).\n\
                 Install from source instead:\n  cargo install --git {}\n\n\
                 (or use the one-line installer from the README, which builds from a release.)",
                repo_home()
            )));
        }
    };
    Ok(triple)
}

fn archive_ext() -> &'static str {
    if cfg!(windows) {
        "zip"
    } else {
        "tar.gz"
    }
}

/// Compare two dotted versions (`0.1.1` vs `0.2.0`). Missing components are 0.
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.')
            .map(|p| {
                // Strip non-numeric suffixes like "0.1.1-beta.1".
                let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };
    let (aa, bb) = (parse(a), parse(b));
    aa.cmp(&bb)
}

/// A URL we will download from: HTTPS always; plain HTTP only for loopback
/// hosts (useful for local release mirrors and tests).
fn url_allowed(s: &str) -> bool {
    match url::Url::parse(s) {
        Ok(u) => {
            let host = u.host_str().unwrap_or("");
            let loopback = u.scheme() == "http"
                && (host == "localhost"
                    || host == "127.0.0.1"
                    || host == "::1"
                    || host == "[::1]"
                    || host.starts_with("127."));
            (u.scheme() == "https" || loopback) && u.username().is_empty() && u.password().is_none()
        }
        Err(_) => false,
    }
}

/// Download `url` to `path`, showing a live progress bar. Content-type checks
/// mirror the SVG downloader so an HTML error page is never saved.
async fn download(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    label: &str,
    ui: &Ui,
) -> Result<u64> {
    if !url_allowed(url) {
        return Err(Error::InvalidUrl(url.to_string()));
    }
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(Error::Http {
            status: response.status().as_u16(),
            detail: format!("download failed for {url}"),
        });
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("text/html") {
        return Err(Error::Download(format!(
            "unexpected content type from server: {content_type}"
        )));
    }

    let total = response.content_length().unwrap_or(0);
    let mut progress = Progress::new(ui, label, total);

    let mut file = std::fs::File::create(path)?;
    let mut remaining = total;
    let mut body = response.bytes_stream();
    let mut written: u64 = 0;
    use futures::StreamExt;
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        written += chunk.len() as u64;
        remaining = remaining.saturating_sub(chunk.len() as u64);
        progress.update(written);
        if remaining == 0 {
            break;
        }
    }
    progress.finish();
    Ok(written)
}

/// Fetch a `.sha256` asset and return the expected hex digest.
async fn fetch_checksum(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(Error::Http {
            status: response.status().as_u16(),
            detail: "could not fetch checksum".to_string(),
        });
    }
    let text = response.text().await?;
    text.split_whitespace()
        .find(|tok| tok.len() == 64 && tok.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(|h| h.to_ascii_lowercase())
        .ok_or_else(|| {
            Error::Other(format!(
                "checksum file had no valid SHA-256 digest: {}",
                url
            ))
        })
}

fn sha256_of(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Extract `archive` into `dest`. Supports the `.tar.gz` and `.zip` layouts
/// produced by the release workflow.
fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    if archive_ext() == "zip" {
        let mut z = zip::ZipArchive::new(file)?;
        z.extract(dest)?;
    } else {
        let dec = GzDecoder::new(file);
        let mut tar = tar::Archive::new(dec);
        tar.unpack(dest)?;
    }
    Ok(())
}

/// Recursively find release binaries (`svgfetch`, optional `.exe`)
/// inside the extracted archive.
fn find_binaries(root: &Path) -> Result<Vec<PathBuf>> {
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let mut found = Vec::new();
    for entry in walkdir(root)? {
        if entry.is_file() {
            let name = entry
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name == format!("svgfetch{ext}")
                || name == format!("svg-fetch{ext}")
                || name == format!("get-svg{ext}")
                || name == format!("getsvg{ext}")
            {
                found.push(entry);
            }
        }
    }
    if found.is_empty() {
        return Err(Error::Other(
            "release archive contained no svgfetch binary".to_string(),
        ));
    }
    Ok(found)
}

fn walkdir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// Put `source` (the fresh binary) in place of `dest`, keeping a `.old`
/// backup long enough to survive on Windows, then deleting it.
#[cfg(not(windows))]
fn install_binary(source: &Path, dest: &Path) -> Result<()> {
    let old = old_path(dest);
    if dest.exists() {
        std::fs::rename(dest, &old)?;
    }
    std::fs::copy(source, dest)?;
    let perms = std::fs::metadata(source)?.permissions();
    std::fs::set_permissions(dest, perms)?;
    if old.exists() {
        let _ = std::fs::remove_file(&old);
    }
    Ok(())
}

#[cfg(windows)]
fn install_binary(source: &Path, dest: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    let old = old_path(dest);
    if dest.exists() {
        std::fs::rename(dest, &old)?;
    }
    std::fs::copy(source, dest)?;
    let perms = std::fs::metadata(source)?.permissions();
    std::fs::set_permissions(dest, perms)?;
    // The running exe may be the `.old` file; deleting it now can fail, so
    // schedule a detached, delayed delete that actually removes it once the
    // process has exited.
    if old.exists() {
        let _ = std::fs::remove_file(&old).or_else(|_| {
            std::process::Command::new("cmd")
                .creation_flags(0x0800_0000 | 0x0000_0008) // CREATE_NO_WINDOW | DETACHED_PROCESS
                .args([
                    "/C",
                    "ping -n 3 127.0.0.1 >nul & del /q",
                    &old.to_string_lossy(),
                ])
                .spawn()
                .map(|_| ())
        });
    }
    Ok(())
}

fn old_path(dest: &Path) -> PathBuf {
    let mut os = dest.as_os_str().to_os_string();
    os.push(".old");
    PathBuf::from(os)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionCheckCache {
    pub last_checked_secs: u64,
    pub latest_version: String,
}

pub fn version_cache_file() -> PathBuf {
    if let Ok(settings) = crate::config::Settings::load() {
        settings.cache_dir().join("version_check.json")
    } else {
        std::env::temp_dir().join("svgfetch_version_check.json")
    }
}

pub fn get_cached_latest_version() -> Option<String> {
    let file = version_cache_file();
    if !file.exists() {
        return None;
    }
    let data = std::fs::read_to_string(file).ok()?;
    let cache: VersionCheckCache = serde_json::from_str(&data).ok()?;
    Some(cache.latest_version)
}

pub fn save_cached_latest_version(version: &str) {
    let file = version_cache_file();
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cache = VersionCheckCache {
        last_checked_secs: now,
        latest_version: version.to_string(),
    };
    if let Ok(data) = serde_json::to_string(&cache) {
        let _ = std::fs::write(file, data);
    }
}

/// Print pip-style update notice to stderr if a newer version is known.
pub fn check_and_print_update_notice() {
    let current = crate::VERSION;
    if let Some(latest) = get_cached_latest_version() {
        if version_cmp(current, &latest) == std::cmp::Ordering::Less {
            let ui = Ui::detect();
            let tag = ui::yellow("[notice]", ui.color);
            eprintln!();
            eprintln!(
                "  {tag} A new release of svgfetch is available: {} -> {}",
                ui::red(&format!("v{current}"), ui.color),
                ui::green(&format!("v{latest}"), ui.color)
            );
            eprintln!(
                "  {tag} To update, run: {}",
                ui::accent("svgfetch update", ui.color)
            );
            eprintln!();
        }
    }
}

/// Check if cache is older than 15 minutes.
pub fn is_version_cache_stale() -> bool {
    let file = version_cache_file();
    if !file.exists() {
        return true;
    }
    if let Ok(data) = std::fs::read_to_string(&file) {
        if let Ok(cache) = serde_json::from_str::<VersionCheckCache>(&data) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            return now.saturating_sub(cache.last_checked_secs) > 15 * 60;
        }
    }
    true
}

async fn fetch_latest_release_tag(client: &reqwest::Client) -> Option<String> {
    let latest_url = format!("{}/releases/latest", api_root());
    let res = client
        .get(&latest_url)
        .header("User-Agent", format!("svgfetch/{}", crate::VERSION))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    let rel: Release = res.json().await.ok()?;
    Some(rel.tag_name.trim_start_matches('v').to_string())
}

async fn is_github_action_in_progress(client: &reqwest::Client) -> bool {
    let url = "https://api.github.com/repos/avdeshjadon/svgfetch/actions/runs?per_page=5";
    if let Ok(res) = client
        .get(url)
        .header("User-Agent", format!("svgfetch/{}", crate::VERSION))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        if let Ok(json) = res.json::<serde_json::Value>().await {
            if let Some(runs) = json.get("workflow_runs").and_then(|v| v.as_array()) {
                for run in runs {
                    let branch = run
                        .get("head_branch")
                        .and_then(|b| b.as_str())
                        .unwrap_or("");
                    let status = run.get("status").and_then(|s| s.as_str()).unwrap_or("");
                    if branch == "main" && (status == "in_progress" || status == "queued") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub async fn run_background_version_check() {
    if !is_version_cache_stale() {
        return;
    }
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent(format!("svgfetch/{} (check)", crate::VERSION))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let main_ver = fetch_upstream_main_version(&client).await;
    let rel_ver = fetch_latest_release_tag(&client).await;

    let candidate = match (main_ver, rel_ver) {
        (Some(m), Some(r)) => {
            if version_cmp(&m, &r) == std::cmp::Ordering::Greater {
                Some(m)
            } else {
                Some(r)
            }
        }
        (Some(m), None) => Some(m),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    };

    if let Some(v) = candidate {
        save_cached_latest_version(&v);
    }
}

pub fn spawn_background_version_check() {
    if !is_version_cache_stale() {
        return;
    }
    tokio::spawn(async {
        run_background_version_check().await;
    });
}

pub fn extract_version_from_cargo_toml(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = \"") || trimmed.starts_with("version=\"") {
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
    }
    None
}

async fn fetch_upstream_main_version(client: &reqwest::Client) -> Option<String> {
    let url = "https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/Cargo.toml";
    let res = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    let text = res.text().await.ok()?;
    extract_version_from_cargo_toml(&text)
}

/// Run the self-update flow. Returns a process exit code.
pub async fn run_update() -> Result<i32> {
    let ui = Ui::detect();
    let current = crate::VERSION;
    let exe = std::env::current_exe().map_err(|_| {
        Error::Other(
            "could not locate the running binary (is it installed on a writable path?)".to_string(),
        )
    })?;
    let bin_dir = exe
        .parent()
        .ok_or_else(|| Error::Other("could not determine the install directory".to_string()))?
        .to_path_buf();

    let triple = target_triple()?;
    let ext = archive_ext();
    let primary_asset = format!("svgfetch-{triple}.{ext}");
    let legacy_asset = format!("get-svg-{triple}.{ext}");

    let client = reqwest::Client::builder()
        .user_agent(format!("svgfetch/{current} (self-update)"))
        .build()?;

    // --- Step 1/4: check upstream main + latest release -------------------
    let spinner = ui.spinner("Checking latest version\u{2026}");
    let raw_main_version = fetch_upstream_main_version(&client).await;
    let latest_url = format!("{}/releases/latest", api_root());
    let release_res = client.get(&latest_url).send().await;
    let release: Option<Release> = match release_res {
        Ok(r) if r.status().is_success() => r.json().await.ok(),
        _ => None,
    };
    let latest = release
        .as_ref()
        .map(|r| r.tag_name.trim_start_matches('v').to_string());
    let check_elapsed = spinner.map(|s| s.finish_min(MIN_STEP)).unwrap_or_default();

    // Scenario 1: Upstream main has a newer version than current
    if let Some(ref main_ver) = raw_main_version {
        if version_cmp(current, main_ver) == std::cmp::Ordering::Less {
            save_cached_latest_version(main_ver);

            let release_is_behind = match &latest {
                Some(rel_ver) => version_cmp(rel_ver, main_ver) == std::cmp::Ordering::Less,
                None => true,
            };

            let action_in_progress = is_github_action_in_progress(&client).await;

            if release_is_behind || action_in_progress {
                ui.render_step(
                    "Checking latest version",
                    1,
                    4,
                    check_elapsed,
                    &[LogLine::notice(format!(
                        "v{main_ver} is building on GitHub Actions"
                    ))],
                );
                ui.notice(&[
                    format!("Version v{main_ver} is currently building on GitHub Actions."),
                    "Wait for some time, version is still in progress.".to_string(),
                    "Please wait an estimated 5 to 10 minutes to compile, then you can update."
                        .to_string(),
                ]);
                return Ok(0);
            }

            if let Some(ref rel) = release {
                let has_platform_asset = rel
                    .assets
                    .iter()
                    .any(|a| a.name == primary_asset || a.name == legacy_asset);
                if !has_platform_asset {
                    ui.render_step(
                        "Checking latest version",
                        1,
                        4,
                        check_elapsed,
                        &[LogLine::notice(format!(
                            "v{main_ver} platform assets still compiling"
                        ))],
                    );
                    ui.notice(&[
                        format!("Version v{main_ver} is currently building on GitHub Actions."),
                        "Wait for some time, version is still in progress (platform assets compiling)."
                            .to_string(),
                        "Please wait an estimated 5 to 10 minutes to compile, then you can update."
                            .to_string(),
                    ]);
                    return Ok(0);
                }
            }
        }
    }

    // Scenario 2: No newer version on main or releases
    let latest_ver_str = match &latest {
        Some(l) => l.clone(),
        None => {
            ui.render_step(
                "Checking latest version",
                1,
                4,
                check_elapsed,
                &[LogLine::skipped("No published releases found")],
            );
            ui.up_to_date(current);
            return Ok(0);
        }
    };

    if version_cmp(current, &latest_ver_str) != std::cmp::Ordering::Less {
        ui.render_step(
            "Checking latest version",
            1,
            4,
            check_elapsed,
            &[LogLine::ok(
                "checked",
                format!("v{current} is the latest release"),
            )],
        );
        ui.up_to_date(current);
        return Ok(0);
    }

    ui.render_step(
        "Checking latest version",
        1,
        4,
        check_elapsed,
        &[LogLine::ok(
            "checked",
            format!("v{current} \u{2192} v{latest_ver_str}"),
        )],
    );

    let release = release.unwrap();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == primary_asset || a.name == legacy_asset)
        .ok_or_else(|| {
            Error::Other(format!(
                "release v{} does not ship {} for your platform yet. Build may still be in progress; please try again in a few minutes.",
                latest_ver_str, primary_asset
            ))
        })?;

    let asset_name = asset.name.clone();
    let checksum_name = format!("{asset_name}.sha256");
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .ok_or_else(|| {
            Error::Other(format!(
                "release v{} is missing checksum {checksum_name}; build may still be finalizing.",
                latest_ver_str
            ))
        })?;

    println!(
        "  {} Updating svgfetch v{current} \u{2192} v{latest_ver_str} (installed in {})",
        ui::dim("\u{00b7}", ui.color),
        ui::tilde(&bin_dir)
    );

    let tmp = tempfile::tempdir()?;
    let archive_path = tmp.path().join(&asset_name);

    // --- Step 2/4: download archive ---------------------------------------
    let expected = fetch_checksum(&client, &checksum_asset.browser_download_url).await?;
    let download_label = format!("svgfetch v{current} \u{2192} v{latest_ver_str}");
    let download_start = Instant::now();
    let bytes = download(
        &client,
        &asset.browser_download_url,
        &archive_path,
        &download_label,
        &ui,
    )
    .await?;
    ui.render_step(
        "Downloading release",
        2,
        4,
        download_start.elapsed(),
        &[LogLine::ok(
            "downloaded",
            format!("Archive received ({}, {asset_name})", format_size(bytes)),
        )],
    );

    // --- Step 3/4: verify checksum ----------------------------------------
    let (digest, verify_elapsed) = {
        let spinner = ui.spinner("Verifying SHA-256\u{2026}");
        let digest = sha256_of(&archive_path);
        let elapsed = spinner.map(|s| s.finish_min(MIN_STEP)).unwrap_or_default();
        (digest, elapsed)
    };
    let got = digest?;
    if got != expected {
        return Err(Error::Other(format!(
            "checksum mismatch (expected {expected}, got {got}) — aborting without touching your install."
        )));
    }
    ui.render_step(
        "Verifying integrity",
        3,
        4,
        verify_elapsed,
        &[LogLine::ok("verified", "SHA-256 integrity verified")],
    );

    // --- Step 4/4: replace binaries ---------------------------------------
    let spinner = ui.spinner("Replacing binary\u{2026}");
    let extract_dir = tmp.path().join("pkg");
    std::fs::create_dir_all(&extract_dir)?;
    extract(&archive_path, &extract_dir)?;

    let fresh = find_binaries(&extract_dir)?;
    let ext_bin = if cfg!(windows) { ".exe" } else { "" };
    let mut installed = Vec::new();
    for name in ["svg", "svgfetch", "svg-fetch", "get-svg", "getsvg"] {
        let target = bin_dir.join(format!("{name}{ext_bin}"));
        if let Some(source) = fresh.iter().find(|p| {
            p.file_name().map(|n| n.to_string_lossy().into_owned())
                == Some(format!("{name}{ext_bin}"))
        }) {
            install_binary(source, &target)?;
            installed.push(
                target
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            );
        } else if name == "svg" && bin_dir.join(format!("svgfetch{ext_bin}")).exists() {
            let main_bin = bin_dir.join(format!("svgfetch{ext_bin}"));
            let _ = std::fs::copy(&main_bin, &target);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
            }
            installed.push(name.to_string());
        }
    }

    for name in ["svg", "svgfetch", "svg-fetch", "get-svg", "getsvg"] {
        let stale = bin_dir.join(format!("{name}{ext_bin}.old"));
        if stale.exists() {
            let _ = std::fs::remove_file(&stale);
        }
    }
    let install_elapsed = spinner.map(|s| s.finish_min(MIN_STEP)).unwrap_or_default();

    save_cached_latest_version(&latest_ver_str);

    ui.render_step(
        "Replacing binary",
        4,
        4,
        install_elapsed,
        &[LogLine::ok(
            "installed",
            format!("Binaries updated successfully ({})", installed.join(", ")),
        )],
    );

    ui.print_box(&[
        format!("Successfully updated svgfetch from v{current} to v{latest_ver_str}!"),
        String::new(),
        "Restart your terminal or run `svgfetch` to start using it.".to_string(),
    ]);
    println!();
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_order() {
        assert_eq!(version_cmp("0.1.1", "0.1.1"), std::cmp::Ordering::Equal);
        assert_eq!(version_cmp("0.1.1", "0.2.0"), std::cmp::Ordering::Less);
        assert_eq!(version_cmp("0.10.0", "0.2.0"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("0.1.1", "0.1.2-beta"), std::cmp::Ordering::Less);
    }

    #[test]
    fn checksum_line_parses_hex() {
        let text = "fc6467fabb6c024e5211e4388b8bf16ad4e203bcbb5c6c52a2e4f58a1af1088a  svgfetch\n";
        let hex = text
            .split_whitespace()
            .find(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap();
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn install_binary_replaces_and_removes_old() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("svgfetch-new");
        let dest = tmp.path().join("svgfetch");
        std::fs::write(&src, b"new-binary").unwrap();
        std::fs::write(&dest, b"old-binary").unwrap();
        install_binary(&src, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"new-binary");
        assert!(!old_path(&dest).exists());
    }
}
