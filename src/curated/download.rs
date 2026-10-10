//! Asset downloading and verification from `svgfetch-icons` CDN.

use reqwest::Client;
use std::path::Path;

use crate::error::{Error, Result};

pub const CDN_BASE_URL: &str = "https://cdn.jsdelivr.net/gh/avdeshjadon/svgfetch-icons@main/logos";
pub const RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/avdeshjadon/svgfetch-icons/main/logos";

/// Download an SVG file from the curated CDN, validate its XML contents, and save it atomically.
pub async fn download_curated_file(
    client: &Client,
    filename: &str,
    target_path: &Path,
    overwrite: bool,
) -> Result<u64> {
    if target_path.exists() && !overwrite {
        return Err(Error::Download(format!(
            "File already exists: {}. Pass --overwrite to replace it.",
            target_path.display()
        )));
    }

    let cdn_url = format!("{}/{}", CDN_BASE_URL, urlencoding_filename(filename));
    let raw_url = format!("{}/{}", RAW_BASE_URL, urlencoding_filename(filename));

    // Try jsDelivr CDN first
    let res = match client.get(&cdn_url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => {
            // Fallback to GitHub raw content
            client.get(&raw_url).send().await.map_err(|e| {
                Error::Download(format!("Failed to fetch {} from CDN: {}", filename, e))
            })?
        }
    };

    if !res.status().is_success() {
        return Err(Error::Download(format!(
            "CDN returned HTTP error {} for {}",
            res.status(),
            filename
        )));
    }

    let bytes = res.bytes().await.map_err(|e| {
        Error::Download(format!(
            "Failed to stream SVG bytes for {}: {}",
            filename, e
        ))
    })?;

    // Validate SVG integrity
    crate::download::file::validate_svg_content(&bytes)?;

    // Ensure parent directory exists
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(Error::Io)?;
    }

    // Atomic write via temporary file
    let tmp_path = target_path.with_extension("tmp.svg");
    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(Error::Io)?;

    tokio::fs::rename(&tmp_path, target_path)
        .await
        .map_err(Error::Io)?;

    Ok(bytes.len() as u64)
}

fn urlencoding_filename(filename: &str) -> String {
    url::form_urlencoded::byte_serialize(filename.as_bytes()).collect()
}
