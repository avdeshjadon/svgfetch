//! Curated brand logo registry backed by `svgfetch-icons` CDN.
//!
//! Provides instant, zero-latency brand matching from the bundled catalog
//! and streams high-resolution vector assets directly from the global jsDelivr edge CDN.

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::models::AssetVariant;

/// Base URL for raw asset delivery via jsDelivr edge CDN.
pub const CDN_BASE_URL: &str = "https://cdn.jsdelivr.net/gh/avdeshjadon/svgfetch-icons@main/logos";

/// Secondary fallback URL directly against GitHub raw content.
pub const RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/avdeshjadon/svgfetch-icons/main/logos";

const BUNDLED_MANIFEST: &str = include_str!("curated_manifest.json");

/// Entry in the curated catalog manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedEntry {
    pub name: String,
    pub shortname: String,
    #[serde(default)]
    pub url: String,
    pub files: Vec<String>,
}

/// Resolved variants available for a curated entry.
#[derive(Debug, Clone)]
pub struct CuratedVariants {
    pub full_file: Option<String>,
    pub icon_file: Option<String>,
    pub all_files: Vec<String>,
}

impl CuratedEntry {
    /// Classify files into Full logo vs Icon mark.
    pub fn variants(&self) -> CuratedVariants {
        let mut icon_file = None;
        let mut full_file = None;

        for file in &self.files {
            let lower = file.to_lowercase();
            if lower.contains("-icon") || lower.contains("_icon") {
                if icon_file.is_none() {
                    icon_file = Some(file.clone());
                }
            } else if full_file.is_none() {
                full_file = Some(file.clone());
            }
        }

        // Fallback: if no full file was found, use the first file
        if full_file.is_none() && !self.files.is_empty() {
            full_file = Some(self.files[0].clone());
        }

        CuratedVariants {
            full_file,
            icon_file,
            all_files: self.files.clone(),
        }
    }
}

/// Retrieve the static curated catalog parsed once from the bundled manifest.
pub fn get_curated_catalog() -> &'static [CuratedEntry] {
    static CATALOG: OnceLock<Vec<CuratedEntry>> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(BUNDLED_MANIFEST).unwrap_or_default())
}

/// Search for a brand in the curated catalog.
pub fn find_curated_brand(query: &str) -> Option<&'static CuratedEntry> {
    let q = query.trim().to_lowercase();
    let q_slug = q.replace(' ', "-").replace('_', "-");
    let catalog = get_curated_catalog();

    // 1. Exact match on shortname
    if let Some(entry) = catalog
        .iter()
        .find(|e| e.shortname.to_lowercase() == q_slug)
    {
        return Some(entry);
    }

    // 2. Exact match on brand name
    if let Some(entry) = catalog.iter().find(|e| e.name.to_lowercase() == q) {
        return Some(entry);
    }

    // 3. Normalized name comparison
    if let Some(entry) = catalog.iter().find(|e| {
        let name_clean = e.name.to_lowercase().replace(' ', "-").replace('_', "-");
        name_clean == q_slug
    }) {
        return Some(entry);
    }

    // 4. Prefix match if query length >= 4
    if q.len() >= 4 {
        if let Some(entry) = catalog.iter().find(|e| {
            e.shortname.to_lowercase().starts_with(&q_slug) || e.name.to_lowercase().starts_with(&q)
        }) {
            return Some(entry);
        }
    }

    None
}

/// Select which filename to download, prompting interactively if both Full and Icon exist.
pub fn select_variant(
    entry: &CuratedEntry,
    variant_flag: Option<AssetVariant>,
    is_interactive: bool,
) -> String {
    let variants = entry.variants();

    // 1. If user explicitly specified variant flag
    if let Some(v) = variant_flag {
        match v {
            AssetVariant::Icon => {
                if let Some(icon) = &variants.icon_file {
                    return icon.clone();
                } else {
                    eprintln!(
                        "[info] Icon mark not available for \"{}\". Downloading standard logo...",
                        entry.name
                    );
                    return variants.full_file.unwrap_or_else(|| entry.files[0].clone());
                }
            }
            AssetVariant::Full | AssetVariant::Default => {
                if let Some(full) = &variants.full_file {
                    return full.clone();
                }
            }
            _ => {}
        }
    }

    // 2. If both full logo and icon mark exist, prompt interactively
    if let (Some(full), Some(icon)) = (variants.full_file.as_ref(), variants.icon_file.as_ref()) {
        if is_interactive && std::io::stdin().is_terminal() {
            eprintln!(
                "[curated] Found \"{}\" in svgfetch-icons library.",
                entry.name
            );
            eprintln!("Select variant to download:");
            eprintln!("  1) Normal logo ({}) [default]", full);
            eprintln!("  2) Icon mark   ({})", icon);
            eprint!("Enter selection [1-2] (press Enter for normal): ");
            let _ = std::io::stderr().flush();

            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_ok() {
                let trimmed = input.trim().to_lowercase();
                if trimmed == "2" || trimmed == "icon" || trimmed == "i" {
                    return icon.clone();
                }
            }
            return full.clone();
        } else {
            return full.clone();
        }
    }

    // 3. Only one variant available
    let chosen = variants.full_file.unwrap_or_else(|| entry.files[0].clone());
    eprintln!(
        "[curated] Found \"{}\" in svgfetch-icons library: {}",
        entry.name, chosen
    );
    chosen
}

/// Download an SVG file from the curated CDN, validate its XML contents, and save it atomically.
pub async fn download_curated_file(
    client: &reqwest::Client,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_loaded_and_non_empty() {
        let catalog = get_curated_catalog();
        assert!(
            catalog.len() >= 1400,
            "Bundled catalog must contain at least 1,400 curated brands"
        );
    }

    #[test]
    fn finds_curated_brands_correctly() {
        assert!(find_curated_brand("instagram").is_some());
        assert!(find_curated_brand("Instagram").is_some());
        assert!(find_curated_brand("github").is_some());
        assert!(find_curated_brand("react").is_some());
        assert!(find_curated_brand("adobe").is_some());
    }

    #[test]
    fn extracts_both_variants_when_present() {
        let instagram = find_curated_brand("instagram").unwrap();
        let variants = instagram.variants();
        assert!(variants.icon_file.is_some());
        assert!(variants.full_file.is_some());
        assert_eq!(variants.icon_file.unwrap(), "instagram-icon.svg");
        assert_eq!(variants.full_file.unwrap(), "instagram.svg");
    }

    #[test]
    fn handles_brands_with_single_variant() {
        let react = find_curated_brand("react").unwrap();
        let variants = react.variants();
        assert_eq!(variants.full_file.unwrap(), "react.svg");
        assert!(variants.icon_file.is_none());
    }
}
