//! In-memory and cached catalog lookup for curated brand assets.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Fallback baseline manifest bundled from repo's `data/curated_manifest.json`.
const BUNDLED_MANIFEST: &str = include_str!("../../data/curated_manifest.json");

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

static CATALOG: RwLock<Option<Vec<CuratedEntry>>> = RwLock::new(None);

/// Retrieve the active curated catalog.
/// Checks the local disk cache first (if fresh), otherwise parses the bundled baseline.
pub fn get_curated_catalog() -> Vec<CuratedEntry> {
    {
        let read = CATALOG.read().unwrap();
        if let Some(cat) = read.as_ref() {
            return cat.clone();
        }
    }

    // Try loading from cached manifest file on disk
    let loaded = crate::curated::sync::load_cached_manifest_from_disk()
        .unwrap_or_else(|| serde_json::from_str(BUNDLED_MANIFEST).unwrap_or_default());

    let mut write = CATALOG.write().unwrap();
    *write = Some(loaded.clone());
    loaded
}

/// Update the in-memory catalog (e.g. after a live sync).
pub fn update_in_memory_catalog(entries: Vec<CuratedEntry>) {
    let mut write = CATALOG.write().unwrap();
    *write = Some(entries);
}

/// Search for a brand in the curated catalog.
pub fn find_curated_brand(query: &str) -> Option<CuratedEntry> {
    let q = query.trim().to_lowercase();
    let q_slug = q.replace(' ', "-").replace('_', "-");
    let catalog = get_curated_catalog();

    // 1. Exact match on shortname
    if let Some(entry) = catalog
        .iter()
        .find(|e| e.shortname.to_lowercase() == q_slug)
    {
        return Some(entry.clone());
    }

    // 2. Exact match on brand name
    if let Some(entry) = catalog.iter().find(|e| e.name.to_lowercase() == q) {
        return Some(entry.clone());
    }

    // 3. Normalized name comparison
    if let Some(entry) = catalog.iter().find(|e| {
        let name_clean = e.name.to_lowercase().replace(' ', "-").replace('_', "-");
        name_clean == q_slug
    }) {
        return Some(entry.clone());
    }

    // 4. Prefix match if query length >= 4
    if q.len() >= 4 {
        if let Some(entry) = catalog.iter().find(|e| {
            e.shortname.to_lowercase().starts_with(&q_slug) || e.name.to_lowercase().starts_with(&q)
        }) {
            return Some(entry.clone());
        }
    }

    None
}
