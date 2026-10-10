//! Search orchestration: cache-aware page fetching shared by the TUI and
//! the non-interactive commands.

use crate::api::provider::AssetProvider;
use crate::cache::{Cache, NS_SEARCH};
use crate::error::Result;
use crate::models::SearchPage;

/// A search page plus whether it came from the local cache.
#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub page: SearchPage,
    pub from_cache: bool,
}

/// Confidence level for a search result match.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum MatchConfidence {
    Low,
    Medium,
    High,
}

impl MatchConfidence {
    pub fn label(&self) -> &'static str {
        match self {
            MatchConfidence::High => "High",
            MatchConfidence::Medium => "Medium",
            MatchConfidence::Low => "Low",
        }
    }
}

/// Compute a deterministic relevance and confidence score for a candidate asset against a query.
pub fn score_asset_match(query: &str, asset: &crate::models::Asset) -> (u32, MatchConfidence) {
    let q_norm = crate::brands::normalize(query);
    let title_clean = asset
        .original_name
        .trim_end_matches(".svg")
        .trim_end_matches(".SVG");
    let t_norm = crate::brands::normalize(title_clean);

    let mut score = 0u32;
    // 1. Exact normalized match
    if t_norm == q_norm {
        score += 100;
    } else if t_norm.starts_with(&q_norm) {
        // 2. Prefix match (e.g. "amazon" -> "amazon logo")
        score += 60;
    } else if t_norm.contains(&q_norm) {
        // 3. Substring match
        score += 40;
    }

    // 4. Token overlap
    let q_tokens: std::collections::HashSet<_> = q_norm.split_whitespace().collect();
    let t_tokens: std::collections::HashSet<_> = t_norm.split_whitespace().collect();
    let overlap = q_tokens.intersection(&t_tokens).count();
    score += (overlap as u32) * 15;

    // 5. SVG mime confirmation bonus
    if asset.mime.as_deref() == Some("image/svg+xml") {
        score += 10;
    }

    let confidence = if score >= 75 {
        MatchConfidence::High
    } else if score >= 35 {
        MatchConfidence::Medium
    } else {
        MatchConfidence::Low
    };

    (score, confidence)
}

/// Cache key for a query/offset/limit triple.
pub fn cache_key(query: &str, offset: u64, limit: u32) -> String {
    // Normalize the *effective* search string (title-boost etc.) into the key
    // so that changing ranking semantics busts stale cache entries instead of
    // silently serving old results.
    let boost = crate::api::wikimedia::WikimediaClient::compose_search(query, None)
        .trim()
        .to_lowercase();
    // Fall back through whitespace collapse just in case the boost was a no-op.
    let normalized = if boost.is_empty() {
        query.trim()
    } else {
        boost.as_str()
    };
    let joined = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{joined}@{offset}:{limit}")
}

/// Fetch one page of results, reading from cache first when enabled.
pub async fn fetch_page<P: AssetProvider>(
    provider: &P,
    cache: &Cache,
    query: &str,
    offset: u64,
    limit: u32,
) -> Result<FetchedPage> {
    fetch_page_opts(provider, cache, query, offset, limit, false).await
}

/// Fetch one page of results with explicit cache bypass option.
pub async fn fetch_page_opts<P: AssetProvider>(
    provider: &P,
    cache: &Cache,
    query: &str,
    offset: u64,
    limit: u32,
    bypass_cache: bool,
) -> Result<FetchedPage> {
    let key = cache_key(query, offset, limit);
    if !bypass_cache {
        if let Some(page) = cache.get::<SearchPage>(NS_SEARCH, &key) {
            return Ok(FetchedPage {
                page,
                from_cache: true,
            });
        }
    }
    let page = provider.search(query, offset, limit).await?;
    cache.put(NS_SEARCH, &key, &page);
    Ok(FetchedPage {
        page,
        from_cache: false,
    })
}

/// Ignore the cache and force a network fetch (for `r` = refresh or `--refresh`).
pub async fn fetch_page_fresh<P: AssetProvider>(
    provider: &P,
    cache: &Cache,
    query: &str,
    offset: u64,
    limit: u32,
) -> Result<FetchedPage> {
    fetch_page_opts(provider, cache, query, offset, limit, true).await
}

/// Collect up to `limit` assets across as many pages as needed.
///
/// Pagination is lazy: pages are fetched only while more assets are needed,
/// so a request for 50 results never pulls 500.
pub async fn collect_assets<P: AssetProvider>(
    provider: &P,
    cache: &Cache,
    query: &str,
    limit: usize,
    per_page: u32,
    mut on_page: impl FnMut(&SearchPage),
) -> Result<Vec<crate::models::Asset>> {
    let per_page = per_page.clamp(1, 500);
    let mut out: Vec<crate::models::Asset> = Vec::new();
    let mut offset = 0u64;
    let mut empty_streak = 0u8;

    while out.len() < limit {
        let fetched = fetch_page(provider, cache, query, offset, per_page).await?;
        let page = fetched.page;
        if page.assets.is_empty() {
            empty_streak += 1;
            if empty_streak >= 2 {
                break;
            }
        } else {
            empty_streak = 0;
        }
        let total = page.assets.len();
        let total_hits = page.total_hits;
        on_page(&page);
        for asset in page.assets {
            if out.len() >= limit {
                break;
            }
            out.push(asset);
        }
        offset += per_page as u64;
        if let Some(total) = total_hits {
            if offset >= total {
                break;
            }
        }
        if total < per_page as usize {
            break;
        }
    }
    Ok(out)
}

/// Case-insensitive license filter. Never invents a license: assets whose
/// license is unknown are excluded when a filter is active.
pub fn filter_by_license(
    assets: Vec<crate::models::Asset>,
    needle: Option<&str>,
) -> Vec<crate::models::Asset> {
    match needle {
        None => assets,
        Some(needle) => {
            let needle = needle.trim().to_lowercase();
            assets
                .into_iter()
                .filter(|a| {
                    a.license
                        .as_ref()
                        .map(|l| l.to_lowercase().contains(&needle))
                        .unwrap_or(false)
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Asset;

    fn asset(name: &str, license: Option<&str>) -> Asset {
        Asset {
            title: format!("File:{name}"),
            page_id: 1,
            original_name: name.to_string(),
            file_name: name.to_string(),
            index: None,
            size_bytes: None,
            mime: None,
            width: None,
            height: None,
            mediatype: None,
            url: None,
            thumb_url: None,
            description_url: None,
            author: None,
            uploader: None,
            license: license.map(str::to_string),
            license_url: None,
            usage_terms: None,
            attribution: None,
            credit: None,
            description: None,
            categories: vec![],
            uploaded_at: None,
            modified_at: None,
            ..Default::default()
        }
    }

    #[test]
    fn license_filter_excludes_unknown_when_active() {
        let assets = vec![
            asset("a.svg", Some("CC0")),
            asset("b.svg", None),
            asset("c.svg", Some("CC BY-SA 4.0")),
        ];
        let filtered = filter_by_license(assets, Some("cc by-sa"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_name, "c.svg");
    }

    #[test]
    fn license_filter_disabled_keeps_everything() {
        let assets = vec![asset("a.svg", None)];
        assert_eq!(filter_by_license(assets, None).len(), 1);
    }

    #[test]
    fn cache_key_is_normalized() {
        assert_eq!(cache_key(" GitHub ", 0, 25), cache_key("github", 0, 25));
    }

    #[test]
    fn cache_key_incorporates_limit_and_offset() {
        assert_ne!(cache_key("github", 0, 25), cache_key("github", 0, 50));
        assert_ne!(cache_key("github", 0, 25), cache_key("github", 25, 25));
    }

    #[test]
    fn score_asset_match_evaluates_confidence() {
        let mut exact = asset("Amazon_logo.svg", None);
        exact.mime = Some("image/svg+xml".into());
        let (score_exact, conf_exact) = score_asset_match("amazon", &exact);
        assert!(score_exact >= 75);
        assert_eq!(conf_exact, MatchConfidence::High);

        let mut prefix = asset("Amazon_Prime_video.svg", None);
        prefix.mime = Some("image/svg+xml".into());
        let (_, conf_prefix) = score_asset_match("amazon", &prefix);
        assert!(conf_prefix >= MatchConfidence::Medium);

        let unrelated = asset("Completely_Unrelated_Image.svg", None);
        let (_, conf_unrelated) = score_asset_match("amazon", &unrelated);
        assert_eq!(conf_unrelated, MatchConfidence::Low);
    }
}
