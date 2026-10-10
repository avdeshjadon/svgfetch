//! Curated brand logo registry backed by `svgfetch-icons` CDN.
//!
//! Provides zero-latency brand matching from the bundled baseline catalog,
//! automatic live synchronization with `svgfetch-icons` CDN, and streams high-resolution
//! vector assets directly from the global jsDelivr edge CDN.

pub mod catalog;
pub mod contribute;
pub mod download;
pub mod sync;

pub use catalog::{
    find_curated_brand, get_curated_catalog, update_in_memory_catalog, CuratedEntry,
    CuratedVariants,
};
pub use contribute::{auto_contribute_brand, classify_brand_category, find_icons_repo_path};
pub use download::{download_curated_file, CDN_BASE_URL, RAW_BASE_URL};
pub use sync::{
    auto_sync_if_needed, is_cache_expired, manifest_cache_path, sync_curated_catalog,
    MANIFEST_CACHE_TTL, MANIFEST_CDN_URL, MANIFEST_RAW_URL,
};

use crate::models::AssetVariant;
use std::io::{IsTerminal, Write};

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
                        "[info] Icon mark not available for \"{}\". Downloading normal logo...",
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
            eprintln!("  1) Normal logo ({}) [default]", leaf_name(full));
            eprintln!("  2) Icon mark   ({})", leaf_name(icon));
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
        entry.name,
        leaf_name(&chosen)
    );
    chosen
}

fn leaf_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
        assert!(variants.icon_file.unwrap().ends_with("instagram-icon.svg"));
        assert!(variants.full_file.unwrap().ends_with("instagram.svg"));
    }

    #[test]
    fn handles_brands_with_single_variant() {
        let react = find_curated_brand("react").unwrap();
        let variants = react.variants();
        assert!(variants.full_file.unwrap().ends_with("react.svg"));
        assert!(variants.icon_file.is_none());
    }
}
