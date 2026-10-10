//! Automatic and manual sync of manifest from `svgfetch-icons` CDN.

use reqwest::Client;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::curated::catalog::{update_in_memory_catalog, CuratedEntry};
use crate::error::{Error, Result};

pub const MANIFEST_CDN_URL: &str =
    "https://cdn.jsdelivr.net/gh/avdeshjadon/svgfetch-icons@main/manifest.json";
pub const MANIFEST_RAW_URL: &str =
    "https://raw.githubusercontent.com/avdeshjadon/svgfetch-icons/main/manifest.json";

/// Cache TTL for manifest: 24 hours.
pub const MANIFEST_CACHE_TTL: Duration = Duration::from_secs(24 * 3600);

/// Path to local cached manifest on disk.
pub fn manifest_cache_path() -> PathBuf {
    if let Ok(settings) = crate::config::Settings::load() {
        settings.cache_dir().join("curated_manifest.json")
    } else {
        std::env::temp_dir().join("svgfetch_curated_manifest.json")
    }
}

/// Load cached manifest from disk if it exists.
pub fn load_cached_manifest_from_disk() -> Option<Vec<CuratedEntry>> {
    let path = manifest_cache_path();
    if !path.exists() {
        return None;
    }

    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Check if the cached manifest on disk is older than the TTL.
pub fn is_cache_expired() -> bool {
    let path = manifest_cache_path();
    if !path.exists() {
        return true;
    }

    if let Ok(metadata) = std::fs::metadata(&path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed > MANIFEST_CACHE_TTL;
            }
        }
    }
    true
}

/// Fetch manifest from CDN / GitHub raw, save to local cache, and update memory.
pub async fn sync_curated_catalog(client: &Client) -> Result<Vec<CuratedEntry>> {
    let res = match client.get(MANIFEST_CDN_URL).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => client
            .get(MANIFEST_RAW_URL)
            .send()
            .await
            .map_err(|e| Error::Download(format!("Failed to fetch manifest: {e}")))?,
    };

    if !res.status().is_success() {
        return Err(Error::Download(format!(
            "Failed to fetch manifest: HTTP {}",
            res.status()
        )));
    }

    let bytes = res
        .bytes()
        .await
        .map_err(|e| Error::Download(format!("Failed to read manifest bytes: {e}")))?;

    let entries: Vec<CuratedEntry> = serde_json::from_slice(&bytes)
        .map_err(|e| Error::Download(format!("Invalid manifest JSON from CDN: {e}")))?;

    // Save to local cache path
    let cache_path = manifest_cache_path();
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache_path, &bytes);

    // Update in-memory catalog
    update_in_memory_catalog(entries.clone());

    Ok(entries)
}

/// Automatically sync manifest from CDN if needed (e.g. cache expired or brand not found).
pub async fn auto_sync_if_needed(client: &Client, force: bool) -> Option<Vec<CuratedEntry>> {
    if force || is_cache_expired() {
        sync_curated_catalog(client).await.ok()
    } else {
        None
    }
}
