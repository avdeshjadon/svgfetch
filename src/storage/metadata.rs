//! Attribution and license metadata export.
//!
//! svgfetch treats licensing as per-asset metadata. It never assumes two
//! Wikimedia Commons files share a license, and never invents one.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::Asset;
use crate::security;

/// Attribution record for one downloaded file, as stored next to assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEntry {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_filename: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    pub downloaded_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_file_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_terms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Machine-readable record of a file's license.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseEntry {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_filename: Option<String>,
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_terms: Option<String>,
    pub verified: bool,
}

/// Machine-readable record of a file's origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_filename: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_file_url: Option<String>,
    pub provider: String,
}

/// Manifest describing a batch/ZIP export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub tool: String,
    pub version: String,
    pub query: String,
    pub created_at: String,
    pub file_count: usize,
    pub note: String,
}

const MANIFEST_NOTE: &str = "Files are unmodified originals from Wikimedia Commons. \
License information is per-file and was recorded at download time; verify it on the \
source page before reuse. svgfetch does not own or license any Wikimedia content.";

impl AttributionEntry {
    pub fn for_asset(asset: &Asset) -> AttributionEntry {
        Self::for_asset_with_details(asset, None, None, None)
    }

    pub fn for_asset_with_details(
        asset: &Asset,
        saved_filename: Option<&str>,
        sha256: Option<&str>,
        size_bytes: Option<u64>,
    ) -> AttributionEntry {
        let license = asset.license_or_unknown();
        let note = if asset.license.is_none() {
            Some(
                "License could not be determined from API metadata. \
Verify licensing on the Wikimedia Commons page before reuse."
                    .to_string(),
            )
        } else if asset.license_url.is_none() {
            Some("No license URL was provided by the API; check the source page.".to_string())
        } else {
            None
        };

        let saved = saved_filename.map(str::to_string);
        let eff_filename = saved
            .clone()
            .unwrap_or_else(|| security::sanitize_filename(&asset.file_name));

        AttributionEntry {
            filename: eff_filename,
            saved_filename: saved,
            source_filename: Some(asset.original_name.clone()),
            source: "Wikimedia Commons".to_string(),
            source_url: asset.description_url.clone(),
            author: asset.author.clone(),
            license,
            license_url: asset.license_url.clone(),
            downloaded_at: Utc::now().to_rfc3339(),
            sha256: sha256.map(str::to_string),
            size_bytes: size_bytes.or(asset.size_bytes),
            original_file_url: asset.url.clone(),
            title: Some(asset.title.clone()),
            attribution_text: asset.attribution.clone(),
            usage_terms: asset.usage_terms.clone(),
            note,
        }
    }
}

fn license_entry(asset: &Asset, saved_name: Option<&str>) -> LicenseEntry {
    let saved = saved_name.map(str::to_string);
    let eff_filename = saved
        .clone()
        .unwrap_or_else(|| security::sanitize_filename(&asset.file_name));
    LicenseEntry {
        filename: eff_filename,
        saved_filename: saved,
        source_filename: Some(asset.original_name.clone()),
        license: asset.license_or_unknown(),
        license_url: asset.license_url.clone(),
        usage_terms: asset.usage_terms.clone(),
        verified: asset.license.is_some() && asset.license_url.is_some(),
    }
}

fn source_entry(asset: &Asset, saved_name: Option<&str>) -> SourceEntry {
    let saved = saved_name.map(str::to_string);
    let eff_filename = saved
        .clone()
        .unwrap_or_else(|| security::sanitize_filename(&asset.file_name));
    SourceEntry {
        filename: eff_filename,
        saved_filename: saved,
        source_filename: Some(asset.original_name.clone()),
        title: asset.title.clone(),
        page_url: asset.description_url.clone(),
        original_file_url: asset.url.clone(),
        provider: "Wikimedia Commons".to_string(),
    }
}

/// The JSON document bodies that accompany a batch/ZIP export.
pub fn metadata_documents(query: &str, assets: &[Asset]) -> Vec<(String, String)> {
    let attribution: Vec<AttributionEntry> =
        assets.iter().map(AttributionEntry::for_asset).collect();
    let licenses: Vec<LicenseEntry> = assets.iter().map(|a| license_entry(a, None)).collect();
    let sources: Vec<SourceEntry> = assets.iter().map(|a| source_entry(a, None)).collect();
    let manifest = Manifest {
        tool: "svgfetch".to_string(),
        version: crate::VERSION.to_string(),
        query: query.to_string(),
        created_at: Utc::now().to_rfc3339(),
        file_count: assets.len(),
        note: MANIFEST_NOTE.to_string(),
    };

    vec![
        (
            "attribution.json".to_string(),
            serde_json::to_string_pretty(&attribution).unwrap_or_else(|_| "[]".into()),
        ),
        (
            "licenses.json".to_string(),
            serde_json::to_string_pretty(&licenses).unwrap_or_else(|_| "[]".into()),
        ),
        (
            "sources.json".to_string(),
            serde_json::to_string_pretty(&sources).unwrap_or_else(|_| "[]".into()),
        ),
        (
            "manifest.json".to_string(),
            serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".into()),
        ),
    ]
}

/// Write attribution files into `dir`, merging with existing metadata files
/// so repeated downloads never drop earlier records.
pub fn write_metadata_dir(dir: &Path, query: &str, assets: &[Asset]) -> Result<Vec<PathBuf>> {
    let records: Vec<(Asset, Option<String>, Option<u64>)> = assets
        .iter()
        .map(|a| (a.clone(), None, a.size_bytes))
        .collect();
    write_metadata_records(dir, query, &records)
}

/// Detailed metadata writer supporting exact saved filename, SHA-256 and byte sizes.
pub fn write_metadata_records(
    dir: &Path,
    query: &str,
    records: &[(Asset, Option<String>, Option<u64>)],
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;

    // 1. Attribution merge
    let attr_path = dir.join("attribution.json");
    let mut merged_attr: Vec<AttributionEntry> = if attr_path.exists() {
        std::fs::read_to_string(&attr_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for (asset, sha, bytes) in records {
        let entry = AttributionEntry::for_asset_with_details(
            asset,
            Some(&asset.file_name),
            sha.as_deref(),
            *bytes,
        );
        merged_attr.retain(|e| e.filename != entry.filename);
        merged_attr.push(entry);
    }
    std::fs::write(&attr_path, serde_json::to_string_pretty(&merged_attr)?)?;

    // 2. Licenses merge
    let lic_path = dir.join("licenses.json");
    let mut merged_lic: Vec<LicenseEntry> = if lic_path.exists() {
        std::fs::read_to_string(&lic_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for (asset, _, _) in records {
        let entry = license_entry(asset, Some(&asset.file_name));
        merged_lic.retain(|e| e.filename != entry.filename);
        merged_lic.push(entry);
    }
    std::fs::write(&lic_path, serde_json::to_string_pretty(&merged_lic)?)?;

    // 3. Sources merge
    let src_path = dir.join("sources.json");
    let mut merged_src: Vec<SourceEntry> = if src_path.exists() {
        std::fs::read_to_string(&src_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for (asset, _, _) in records {
        let entry = source_entry(asset, Some(&asset.file_name));
        merged_src.retain(|e| e.filename != entry.filename);
        merged_src.push(entry);
    }
    std::fs::write(&src_path, serde_json::to_string_pretty(&merged_src)?)?;

    // 4. Manifest update
    let manifest_path = dir.join("manifest.json");
    let manifest = Manifest {
        tool: "svgfetch".to_string(),
        version: crate::VERSION.to_string(),
        query: query.to_string(),
        created_at: Utc::now().to_rfc3339(),
        file_count: merged_attr.len(),
        note: MANIFEST_NOTE.to_string(),
    };
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(vec![attr_path, lic_path, src_path, manifest_path])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Asset;

    fn asset(name: &str, license: Option<&str>) -> Asset {
        Asset {
            title: format!("File:{name}"),
            page_id: 7,
            original_name: name.to_string(),
            file_name: name.to_string(),
            index: None,
            size_bytes: Some(100),
            mime: Some("image/svg+xml".into()),
            width: None,
            height: None,
            mediatype: None,
            url: Some(format!(
                "https://upload.wikimedia.org/wikipedia/commons/{name}"
            )),
            thumb_url: None,
            description_url: Some(format!("https://commons.wikimedia.org/wiki/File:{name}")),
            author: Some("Test Author".into()),
            uploader: None,
            license: license.map(str::to_string),
            license_url: license.map(|_| "https://creativecommons.org/licenses/by-sa/4.0/".into()),
            usage_terms: None,
            attribution: None,
            credit: None,
            description: None,
            categories: vec!["Icons".into()],
            uploaded_at: None,
            modified_at: None,
            ..Default::default()
        }
    }

    #[test]
    fn attribution_never_invents_license() {
        let e = AttributionEntry::for_asset(&asset("x.svg", None));
        assert_eq!(e.license, "Unknown");
        assert!(e.note.is_some());
        assert!(e.license_url.is_none());
    }

    #[test]
    fn attribution_records_known_license() {
        let e = AttributionEntry::for_asset(&asset("x.svg", Some("CC BY-SA 4.0")));
        assert_eq!(e.license, "CC BY-SA 4.0");
        assert!(e.license_url.is_some());
        assert!(e.note.is_none());
        assert_eq!(e.source, "Wikimedia Commons");
        assert!(e.original_file_url.is_some());
    }

    #[test]
    fn documents_are_valid_json() {
        let docs = metadata_documents("github", &[asset("a.svg", Some("CC0"))]);
        assert_eq!(docs.len(), 4);
        for (name, body) in docs {
            assert!(
                serde_json::from_str::<serde_json::Value>(&body).is_ok(),
                "{name} invalid"
            );
        }
    }

    #[test]
    fn metadata_dir_merges_attribution() {
        let dir = tempfile::tempdir().unwrap();
        write_metadata_dir(dir.path(), "q1", &[asset("a.svg", None)]).unwrap();
        write_metadata_dir(
            dir.path(),
            "q2",
            &[asset("a.svg", Some("CC0")), asset("b.svg", None)],
        )
        .unwrap();
        let raw_attr = std::fs::read_to_string(dir.path().join("attribution.json")).unwrap();
        let entries: Vec<AttributionEntry> = serde_json::from_str(&raw_attr).unwrap();
        assert_eq!(
            entries.len(),
            2,
            "duplicates must be replaced, not appended"
        );
        let raw_lic = std::fs::read_to_string(dir.path().join("licenses.json")).unwrap();
        let lics: Vec<LicenseEntry> = serde_json::from_str(&raw_lic).unwrap();
        assert_eq!(lics.len(), 2, "licenses must merge without loss");

        let raw_src = std::fs::read_to_string(dir.path().join("sources.json")).unwrap();
        let srcs: Vec<SourceEntry> = serde_json::from_str(&raw_src).unwrap();
        assert_eq!(srcs.len(), 2, "sources must merge without loss");

        assert!(dir.path().join("manifest.json").exists());
    }

    #[test]
    fn attribution_records_saved_and_source_filenames() {
        let a = asset("SourceLogo.svg", Some("CC0"));
        let entry = AttributionEntry::for_asset_with_details(
            &a,
            Some("TargetLogo-1.svg"),
            Some("abcdef123456"),
            Some(1024),
        );
        assert_eq!(entry.filename, "TargetLogo-1.svg");
        assert_eq!(entry.saved_filename.as_deref(), Some("TargetLogo-1.svg"));
        assert_eq!(entry.source_filename.as_deref(), Some("SourceLogo.svg"));
        assert_eq!(entry.sha256.as_deref(), Some("abcdef123456"));
        assert_eq!(entry.size_bytes, Some(1024));
    }
}
