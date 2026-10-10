//! Automatic contribution and synchronization of new assets to `svgfetch-icons`.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::AssetVariant;

/// Locate the local `svgfetch-icons` repository if available.
pub fn find_icons_repo_path() -> Option<PathBuf> {
    // 1. Environment variable override
    if let Ok(env_path) = std::env::var("SVGFETCH_ICONS_REPO") {
        let p = PathBuf::from(env_path);
        if p.join("logos").exists() {
            return Some(p);
        }
    }

    // 2. Relative sibling directory from current working dir
    if let Ok(cwd) = std::env::current_dir() {
        let sibling = cwd.parent().map(|p| p.join("svgfetch-icons"));
        if let Some(s) = sibling {
            if s.join("logos").exists() {
                return Some(s);
            }
        }
    }

    // 3. Known standard location on developer machine
    let standard = PathBuf::from("/Users/avdeshjadon/svgfetch-icons");
    if standard.join("logos").exists() {
        return Some(standard);
    }

    None
}

/// Classify a brand into its category folder.
pub fn classify_brand_category(brand_name: &str) -> &'static str {
    let b = brand_name.to_lowercase();
    if b.contains("google") || b.contains("android") || b.contains("chrome") || b.contains("youtube") {
        "google"
    } else if b.contains("microsoft") || b.contains("azure") || b.contains("windows") || b.contains("vscode") {
        "microsoft"
    } else if b.contains("adobe") || b.contains("photoshop") || b.contains("illustrator") {
        "adobe"
    } else if b.contains("meta") || b.contains("facebook") || b.contains("instagram") || b.contains("whatsapp") {
        "meta"
    } else if b.contains("apple") || b.contains("safari") || b.contains("swift") {
        "apple"
    } else if b.contains("amazon") || b.contains("aws") {
        "amazon"
    } else if b.contains("myntra")
        || b.contains("meesho")
        || b.contains("flipkart")
        || b.contains("shopify")
        || b.contains("stripe")
        || b.contains("paypal")
        || b.contains("ebay")
        || b.contains("store")
        || b.contains("shop")
        || b.contains("pay")
        || b.contains("mart")
        || b.contains("commerce")
    {
        "e-commerce"
    } else if b.contains("ai") || b.contains("gpt") || b.contains("claude") || b.contains("bot") || b.contains("neural") {
        "ai"
    } else if b.contains("db") || b.contains("sql") || b.contains("data") || b.contains("redis") || b.contains("mongo") {
        "databases"
    } else if b.contains("cloud") || b.contains("docker") || b.contains("kube") || b.contains("deploy") || b.contains("host") {
        "devops"
    } else if b.contains("social") || b.contains("chat") || b.contains("talk") || b.contains("meet") {
        "social"
    } else if b.contains("code") || b.contains("lang") || b.contains("script") {
        "languages"
    } else {
        "general"
    }
}

/// Automatically contribute a newly downloaded SVG to the `svgfetch-icons` repo,
/// update manifest, commit and push to git, and sync with local project.
pub async fn auto_contribute_brand(
    brand_name: &str,
    svg_bytes: &[u8],
    variant: AssetVariant,
) -> bool {
    let repo_dir = match find_icons_repo_path() {
        Some(p) => p,
        None => return false,
    };

    let category = classify_brand_category(brand_name);
    let brand_slug = crate::resolution::brands::slugify(brand_name);
    let filename = if variant == AssetVariant::Icon {
        format!("{brand_slug}-icon.svg")
    } else {
        format!("{brand_slug}.svg")
    };

    let target_dir = repo_dir.join("logos").join(category);
    let target_path = target_dir.join(&filename);

    if target_path.exists() {
        return false;
    }

    if std::fs::create_dir_all(&target_dir).is_err() {
        return false;
    }

    if std::fs::write(&target_path, svg_bytes).is_err() {
        return false;
    }

    eprintln!(
        "[curated] Auto-saved \"{}\" to svgfetch-icons (logos/{}/{}).",
        brand_name, category, filename
    );

    // Record addition in CONTRIBUTIONS.md log with exact timestamp and path
    record_contribution_log(&repo_dir, brand_name, category, &filename, variant);

    // Run manifest generator in svgfetch-icons
    let manifest_script = repo_dir.join("scripts").join("generate-manifest.js");
    if manifest_script.exists() {
        let _ = Command::new("node")
            .arg(&manifest_script)
            .current_dir(&repo_dir)
            .output();
    }

    // Sync to disk cache and data/curated_manifest.json
    let manifest_source = repo_dir.join("manifest.json");
    if manifest_source.exists() {
        if let Ok(manifest_content) = std::fs::read_to_string(&manifest_source) {
            // Update in-memory catalog
            if let Ok(entries) = serde_json::from_str(&manifest_content) {
                crate::curated::catalog::update_in_memory_catalog(entries);
            }
            // Update persistent cache on disk so future runs see it instantly
            let cache_path = crate::curated::sync::manifest_cache_path();
            if let Some(parent) = cache_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&cache_path, &manifest_content);

            // Update data/curated_manifest.json if repo root exists
            let local_data_manifest = Path::new("data/curated_manifest.json");
            if local_data_manifest.exists() {
                let _ = std::fs::copy(&manifest_source, local_data_manifest);
            }
        }
    }

    // Git add, commit, and push in svgfetch-icons repository
    let commit_msg = format!("feat(icons): auto-curate {} ({})", brand_name, category);
    let _ = Command::new("git")
        .args(["add", "-A"])
        .current_dir(&repo_dir)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(&repo_dir)
        .output();

    // Spawn push in background so user doesn't wait
    let repo_dir_clone = repo_dir.clone();
    tokio::spawn(async move {
        let _ = Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&repo_dir_clone)
            .output();
    });

    eprintln!("[sync] Manifest updated and pushed to svgfetch-icons repository.");
    true
}

/// Append an entry to CONTRIBUTIONS.md tracking table in `svgfetch-icons`.
fn record_contribution_log(
    repo_dir: &Path,
    brand_name: &str,
    category: &str,
    filename: &str,
    variant: AssetVariant,
) {
    let log_path = repo_dir.join("CONTRIBUTIONS.md");
    let now_str = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let variant_str = if variant == AssetVariant::Icon {
        "Icon Mark"
    } else {
        "Full Logo"
    };
    let rel_path = format!("logos/{}/{}", category, filename);

    let table_row = format!(
        "| {} | {} | {} | `{}` | {} | Wikimedia Commons (Auto-Curated) |\n",
        now_str, brand_name, category, rel_path, variant_str
    );

    if !log_path.exists() {
        let initial_content = format!(
            "# Asset Contribution & Curation Log\n\n\
            This document tracks all new vector assets curated into the `svgfetch-icons` library.\n\n\
            ## Curated Additions\n\n\
            | Date & Time (UTC) | Brand | Category | File Path | Variant | Source |\n\
            | :--- | :--- | :--- | :--- | :--- | :--- |\n\
            {}",
            table_row
        );
        let _ = std::fs::write(&log_path, initial_content);
    } else {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&log_path) {
            let _ = file.write_all(table_row.as_bytes());
        }
    }
}

