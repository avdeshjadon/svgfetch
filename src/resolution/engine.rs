//! Exact entity resolution, variant parsing, and semantic ranking engine.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::api::AssetProvider;
use crate::brands::{self, BrandRegistry};
use crate::cache::Cache;
use crate::error::Result;
use crate::models::{Asset, AssetVariant};
use crate::search::{self, MatchConfidence};

/// Fully resolved asset metadata record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Raw or cleaned user query string.
    pub query: String,
    /// Canonical resolved entity name (e.g. "Instagram", "Docker", "Facebook").
    pub resolved_entity: String,
    /// Categorization of the resolved entity.
    pub entity_type: String,
    /// Asset category (e.g. "brand_logo", "mascot", "icon").
    pub asset_type: String,
    /// Asset variant resolved (e.g. default, icon, wordmark, full, mascot).
    pub variant: AssetVariant,
    /// Selected asset title or filename.
    pub selected_asset: String,
    /// Complete Wikimedia Asset record.
    pub asset: Asset,
    /// Resolution confidence percentage (0-100).
    pub confidence: u32,
    /// High/Medium/Low confidence classification.
    pub match_confidence: MatchConfidence,
    /// Human-readable explanation of why this asset was chosen.
    pub match_reason: String,
    /// Whether this was resolved from the curated registry.
    pub is_curated: bool,
    /// List of known/detected available variants for this entity.
    pub available_variants: Vec<AssetVariant>,
}

/// Parse raw query string and extract any embedded variant words while honoring explicit CLI flags.
///
/// If an explicit CLI flag is provided, it always overrides query-embedded variant words.
/// In verbose mode, a note is logged if the query specified a different variant.
pub fn parse_query_and_variant(
    raw_query: &str,
    explicit_flag: Option<AssetVariant>,
    verbose: bool,
) -> (String, AssetVariant) {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return (
            String::new(),
            explicit_flag.unwrap_or(AssetVariant::Default),
        );
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();

    // 1. Try multi-word variant suffix (e.g. "text logo", "full logo")
    let mut detected_variant = None;
    let mut entity_words = words.clone();

    if words.len() >= 3 {
        let two_word_suffix = format!("{}-{}", words[words.len() - 2], words[words.len() - 1]);
        if let Some(v) = AssetVariant::from_str_loose(&two_word_suffix) {
            detected_variant = Some(v);
            entity_words = words[..words.len() - 2].to_vec();
        }
    }

    // 2. Try single-word suffix (e.g. "instagram wordmark", "linux mascot")
    if detected_variant.is_none() && words.len() >= 2 {
        let last = words[words.len() - 1];
        if let Some(v) = AssetVariant::from_str_loose(last) {
            detected_variant = Some(v);
            entity_words = words[..words.len() - 1].to_vec();
        }
    }

    // 3. Try single-word prefix if words >= 2 (e.g. "wordmark instagram")
    if detected_variant.is_none() && words.len() >= 2 {
        let first = words[0];
        if let Some(v) = AssetVariant::from_str_loose(first) {
            detected_variant = Some(v);
            entity_words = words[1..].to_vec();
        }
    }

    let clean_entity = if entity_words.is_empty() {
        trimmed.to_string()
    } else {
        entity_words.join(" ")
    };

    // Explicit flag has strict priority
    match (explicit_flag, detected_variant) {
        (Some(flag_val), Some(query_val)) => {
            if flag_val != query_val && verbose {
                eprintln!(
                    "\x1b[33m\u{2139} Notice:\x1b[0m Query variant '{}' overridden by --variant {}",
                    query_val.as_str(),
                    flag_val.as_str()
                );
            }
            (clean_entity, flag_val)
        }
        (Some(flag_val), None) => (clean_entity, flag_val),
        (None, Some(query_val)) => (clean_entity, query_val),
        (None, None) => (clean_entity, AssetVariant::Default),
    }
}

/// Compose controlled search terms for variant-aware Wikimedia search expansion.
pub fn compose_variant_search_terms(entity: &str, variant: AssetVariant) -> Vec<String> {
    let norm = brands::normalize(entity);
    match norm.as_str() {
        "linux" => match variant {
            AssetVariant::Mascot => vec![
                "Tux.svg".to_string(),
                "Linux mascot Tux".to_string(),
                "Tux Linux".to_string(),
            ],
            AssetVariant::Icon => vec!["Linux logo".to_string(), "Linux branding".to_string()],
            _ => vec![
                "Linux logo".to_string(),
                "Linux branding".to_string(),
                "Linux".to_string(),
            ],
        },
        "tux" => vec![
            "Tux.svg".to_string(),
            "Tux mascot".to_string(),
            "Tux Linux".to_string(),
        ],
        "kali linux" | "kali" => match variant {
            AssetVariant::Mascot => vec![
                "Kali Linux Dragon".to_string(),
                "Kali dragon icon".to_string(),
                "Kali Linux mascot".to_string(),
            ],
            _ => vec![
                "Kali Linux logo".to_string(),
                "Kali Linux 2.0 wordmark".to_string(),
                "Kali Linux".to_string(),
            ],
        },
        "docker" => match variant {
            AssetVariant::Icon => vec![
                "Docker container engine logo".to_string(),
                "Docker icon".to_string(),
                "Docker whale logo".to_string(),
            ],
            AssetVariant::Wordmark => vec![
                "Docker Wordmark".to_string(),
                "Docker Logo".to_string(),
                "Docker logotype".to_string(),
            ],
            AssetVariant::Full => vec![
                "Docker Logo".to_string(),
                "Docker brand lockup".to_string(),
                "Docker full logo".to_string(),
            ],
            _ => vec![
                "Docker Logo".to_string(),
                "Docker container engine logo".to_string(),
                "Docker logo".to_string(),
            ],
        },
        "instagram" => match variant {
            AssetVariant::Icon => vec![
                "Instagram logo 2016".to_string(),
                "Instagram icon".to_string(),
                "Instagram symbol".to_string(),
            ],
            AssetVariant::Wordmark => vec![
                "Instagram wordmark".to_string(),
                "Instagram wordmark 2016".to_string(),
                "Instagram logotype".to_string(),
            ],
            AssetVariant::Full => vec![
                "Instagram logo and wordmark".to_string(),
                "Instagram logo 2016".to_string(),
                "Instagram lockup".to_string(),
            ],
            _ => vec![
                "Instagram logo 2016".to_string(),
                "Instagram logo".to_string(),
                "Instagram brand".to_string(),
            ],
        },
        "github" => match variant {
            AssetVariant::Icon => vec![
                "Octicons mark github".to_string(),
                "GitHub mark".to_string(),
                "GitHub icon".to_string(),
            ],
            AssetVariant::Wordmark => vec![
                "GitHub logo 2013".to_string(),
                "GitHub wordmark".to_string(),
                "GitHub logotype".to_string(),
            ],
            AssetVariant::Full => vec![
                "GitHub logo 2013".to_string(),
                "GitHub lockup".to_string(),
                "GitHub full logo".to_string(),
            ],
            _ => vec![
                "Octicons mark github".to_string(),
                "GitHub logo 2013".to_string(),
                "GitHub logo".to_string(),
            ],
        },
        "facebook" => match variant {
            AssetVariant::Icon => vec![
                "Facebook logo 2019".to_string(),
                "Facebook icon".to_string(),
            ],
            AssetVariant::Wordmark => vec![
                "Facebook New Logo 2015".to_string(),
                "Facebook wordmark".to_string(),
            ],
            _ => vec![
                "Facebook logo 2019".to_string(),
                "Facebook logo".to_string(),
            ],
        },
        "python" => match variant {
            AssetVariant::Icon => vec!["Python logo notext".to_string(), "Python icon".to_string()],
            AssetVariant::Wordmark | AssetVariant::Full => vec![
                "Python logo and wordmark".to_string(),
                "Python wordmark".to_string(),
            ],
            _ => vec![
                "Python logo notext".to_string(),
                "Python logo and wordmark".to_string(),
            ],
        },
        _ => match variant {
            AssetVariant::Default => vec![
                format!("{entity} logo"),
                format!("{entity} brand"),
                entity.to_string(),
            ],
            AssetVariant::Icon => vec![
                format!("{entity} icon"),
                format!("{entity} logo icon"),
                format!("{entity} symbol"),
                format!("{entity} mark"),
            ],
            AssetVariant::Wordmark => vec![
                format!("{entity} wordmark"),
                format!("{entity} logotype"),
                format!("{entity} text logo"),
                format!("{entity} word logo"),
            ],
            AssetVariant::Full => vec![
                format!("{entity} lockup"),
                format!("{entity} logo wordmark"),
                format!("{entity} full logo"),
                format!("{entity} brand lockup"),
            ],
            AssetVariant::Mascot => vec![
                format!("{entity} mascot"),
                format!("{entity} character"),
                format!("{entity} avatar"),
            ],
        },
    }
}

/// Compute semantic score and match reason for an asset against an entity and variant.
pub fn score_asset_variant_match(
    entity: &str,
    variant: AssetVariant,
    asset: &Asset,
) -> (i32, MatchConfidence, String) {
    let mut score = 0i32;
    let mut reasons = Vec::new();

    let e_norm = brands::normalize(entity);
    let title_clean = asset
        .original_name
        .trim_end_matches(".svg")
        .trim_end_matches(".SVG");
    let t_norm = brands::normalize(title_clean);

    let desc = asset.description.as_deref().unwrap_or("");
    let d_norm = brands::normalize(desc);

    let mut cat_str = String::new();
    for c in &asset.categories {
        cat_str.push(' ');
        cat_str.push_str(&brands::normalize(c));
    }

    // --- POSITIVE SIGNALS: ENTITY ---
    if t_norm == e_norm {
        score += 100;
        reasons.push("Exact entity title match");
    } else if t_norm.starts_with(&e_norm) {
        score += 70;
        reasons.push("Entity prefix match");
    } else if t_norm.contains(&e_norm) {
        score += 45;
        reasons.push("Entity name in title");
    } else {
        // Token overlap
        let e_tokens: HashSet<_> = e_norm.split_whitespace().collect();
        let t_tokens: HashSet<_> = t_norm.split_whitespace().collect();
        let overlap = e_tokens.intersection(&t_tokens).count();
        if overlap == e_tokens.len() && !e_tokens.is_empty() {
            score += 55;
            reasons.push("All entity tokens matched");
        } else if overlap > 0 {
            score += (overlap as i32) * 15;
        }
    }

    // --- NEGATIVE SIGNALS: WRONG ENTITY (Absolute Priority) ---
    if e_norm == "facebook" {
        // Facebook must NEVER match Meta Platforms
        if (t_norm.contains("meta") || d_norm.contains("meta platforms"))
            && !t_norm.contains("facebook")
        {
            score -= 250;
        }
    } else if e_norm == "instagram" {
        if (t_norm.contains("meta") || d_norm.contains("meta platforms"))
            && !t_norm.contains("instagram")
        {
            score -= 250;
        }
    } else if e_norm == "github" {
        if t_norm.contains("gitlab") {
            score -= 250;
        }
    } else if e_norm == "kali linux" || e_norm == "kali" {
        // Must NOT match generic Linux or other distros unless Kali is present
        if !t_norm.contains("kali") {
            score -= 250;
        }
    } else if e_norm == "linux" {
        // Linux branding must NOT resolve to Tux unless variant == Mascot
        if variant != AssetVariant::Mascot && t_norm.contains("tux") && !t_norm.contains("logo") {
            score -= 100;
        }
        if t_norm.contains("containers") || t_norm.contains("linux containers") {
            score -= 80;
        }
    } else if e_norm == "docker" && t_norm.contains("whale") && !t_norm.contains("docker") {
        score -= 150;
    }

    // --- POSITIVE SIGNALS: VARIANT MATCHING ---
    match variant {
        AssetVariant::Default => {
            if t_norm.contains("logo") || t_norm.contains("brand") {
                score += 35;
                reasons.push("Canonical logo indicator");
            }
        }
        AssetVariant::Icon => {
            if t_norm.contains("icon") || t_norm.contains("symbol") || t_norm.contains("mark") {
                score += 80;
                reasons.push("Icon variant keywords matched in title");
            } else if t_norm.contains("whale") && e_norm == "docker" {
                score += 75;
                reasons.push("Docker whale icon match");
            } else if t_norm.contains("camera") && e_norm == "instagram" {
                score += 75;
                reasons.push("Instagram camera icon match");
            } else if d_norm.contains("icon") || cat_str.contains("icon") {
                score += 40;
                reasons.push("Icon metadata indicator");
            }

            // Negative variant penalty
            if t_norm.contains("wordmark") || t_norm.contains("logotype") {
                score -= 70;
            }
        }
        AssetVariant::Wordmark => {
            if t_norm.contains("wordmark")
                || t_norm.contains("logotype")
                || t_norm.contains("text logo")
            {
                score += 85;
                reasons.push("Wordmark variant keywords matched in title");
            } else if d_norm.contains("wordmark") || cat_str.contains("wordmark") {
                score += 45;
                reasons.push("Wordmark metadata indicator");
            }

            // Negative variant penalty
            if t_norm.contains("icon only")
                || t_norm.contains("symbol only")
                || t_norm.contains("without text")
            {
                score -= 70;
            }
        }
        AssetVariant::Full => {
            if t_norm.contains("lockup")
                || t_norm.contains("logo and wordmark")
                || t_norm.contains("full logo")
                || t_norm.contains("complete")
            {
                score += 80;
                reasons.push("Full lockup keywords matched in title");
            } else if t_norm.contains("logo") && !t_norm.contains("icon only") {
                score += 40;
                reasons.push("Standard logo lockup candidate");
            }
        }
        AssetVariant::Mascot => {
            if t_norm.contains("mascot")
                || t_norm.contains("character")
                || (e_norm == "linux" && t_norm.contains("tux"))
                || (e_norm == "kali linux" && t_norm.contains("dragon"))
            {
                score += 90;
                reasons.push("Mascot asset match");
            } else if d_norm.contains("mascot") {
                score += 45;
                reasons.push("Mascot metadata indicator");
            }

            if t_norm.contains("wordmark") {
                score -= 70;
            }
        }
    }

    // --- POSITIVE FORMAT CONFIRMATION ---
    if asset.mime.as_deref() == Some("image/svg+xml") {
        score += 15;
    }

    // --- NEGATIVE GENERAL NOISE SIGNALS ---
    let noise = [
        "screenshot",
        "photo",
        "fanart",
        "fan art",
        "wallpaper",
        "mockup",
        "diagram",
        "poster",
        "tshirt",
        "t-shirt",
        "stamp",
    ];
    for n in noise {
        if t_norm.contains(n) {
            score -= 100;
            break;
        }
    }

    let clamped = score.clamp(0, 100) as u32;
    let confidence = if clamped >= 75 {
        MatchConfidence::High
    } else if clamped >= 40 {
        MatchConfidence::Medium
    } else {
        MatchConfidence::Low
    };

    let reason_summary = if !reasons.is_empty() {
        reasons.join(", ")
    } else {
        "Semantic similarity score".to_string()
    };

    (clamped as i32, confidence, reason_summary)
}

/// Central resolution engine: resolves entity and variant against curated registry and Wikimedia search.
pub async fn resolve_entity_and_variant<P: AssetProvider>(
    provider: &P,
    cache: &Cache,
    raw_query: &str,
    explicit_flag: Option<AssetVariant>,
    refresh: bool,
    verbose: bool,
) -> Result<Option<ResolutionResult>> {
    let (entity, variant) = parse_query_and_variant(raw_query, explicit_flag, verbose);
    let entity_trim = entity.trim();
    if entity_trim.is_empty() {
        return Ok(None);
    }

    let registry = BrandRegistry::load();

    // 1. Curated Brand Registry check
    if let Some(brand) = registry.resolve(entity_trim) {
        if let Some(curated_file) = brand.file_for_variant(variant) {
            let asset_opt = if let Some(a) = provider.get_asset(curated_file).await? {
                Some(a)
            } else {
                let found =
                    search::fetch_page_opts(provider, cache, curated_file, 0, 5, refresh).await?;
                found.page.assets.into_iter().next()
            };

            if let Some(mut a) = asset_opt {
                let confidence_pct = 97u32;
                let reason = if variant == AssetVariant::Default {
                    format!(
                        "Exact {} entity match with canonical logo candidate.",
                        brand.name
                    )
                } else {
                    format!(
                        "Exact {} entity + {} variant.",
                        brand.name,
                        variant.as_str()
                    )
                };

                let asset_type = match variant {
                    AssetVariant::Mascot => "mascot",
                    AssetVariant::Icon => "brand_icon",
                    AssetVariant::Wordmark => "brand_wordmark",
                    AssetVariant::Full => "brand_lockup",
                    AssetVariant::Default => "brand_logo",
                }
                .to_string();

                a.query = Some(raw_query.to_string());
                a.resolved_entity = Some(brand.name.clone());
                a.entity_type = Some("brand".to_string());
                a.asset_type = Some(asset_type.clone());
                a.variant = Some(variant.as_str().to_string());
                a.selected_asset = Some(a.original_name.clone());
                a.confidence = Some(confidence_pct);
                a.match_reason = Some(reason.clone());

                return Ok(Some(ResolutionResult {
                    query: raw_query.to_string(),
                    resolved_entity: brand.name.clone(),
                    entity_type: "brand".to_string(),
                    asset_type,
                    variant,
                    selected_asset: a.original_name.clone(),
                    asset: a,
                    confidence: confidence_pct,
                    match_confidence: MatchConfidence::High,
                    match_reason: reason,
                    is_curated: true,
                    available_variants: brand.available_variants(),
                }));
            }
        }
    }

    // 2. Fallback to Variant-Aware Wikimedia Search Expansion & Semantic Ranking
    let search_terms = compose_variant_search_terms(entity_trim, variant);
    let mut candidates: Vec<Asset> = Vec::new();
    let mut seen_titles = HashSet::new();

    for term in &search_terms {
        // Variant-aware query string to avoid cache pollution
        let query_with_type = format!("{term} filemime:image/svg+xml");
        let fetched =
            search::fetch_page_opts(provider, cache, &query_with_type, 0, 10, refresh).await;

        if let Ok(page) = fetched {
            for a in page.page.assets {
                if seen_titles.insert(a.title.clone()) {
                    candidates.push(a);
                }
            }
        }

        // Early termination if we already have strong candidates
        if candidates.len() >= 15 {
            break;
        }
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    // 3. Score all candidates using semantic ranking
    let mut scored: Vec<(Asset, i32, MatchConfidence, String)> = candidates
        .into_iter()
        .map(|a| {
            let (score, conf, reason) = score_asset_variant_match(entity_trim, variant, &a);
            (a, score, conf, reason)
        })
        .collect();

    // Sort descending by score
    scored.sort_by_key(|b| std::cmp::Reverse(b.1));

    if let Some((mut best_asset, score, conf, reason)) = scored.into_iter().next() {
        if score < 35 || conf == MatchConfidence::Low {
            return Ok(None);
        }

        let asset_type = match variant {
            AssetVariant::Mascot => "mascot",
            AssetVariant::Icon => "brand_icon",
            AssetVariant::Wordmark => "brand_wordmark",
            AssetVariant::Full => "brand_lockup",
            AssetVariant::Default => "brand_logo",
        }
        .to_string();

        let display_entity = if let Some(first_letter) = entity_trim.chars().next() {
            let mut c = first_letter.to_uppercase().to_string();
            c.push_str(&entity_trim[first_letter.len_utf8()..]);
            c
        } else {
            entity_trim.to_string()
        };

        let final_reason = format!(
            "Exact {} entity + {} variant ({})",
            display_entity,
            variant.as_str(),
            reason
        );

        best_asset.query = Some(raw_query.to_string());
        best_asset.resolved_entity = Some(display_entity.clone());
        best_asset.entity_type = Some("brand".to_string());
        best_asset.asset_type = Some(asset_type.clone());
        best_asset.variant = Some(variant.as_str().to_string());
        best_asset.selected_asset = Some(best_asset.original_name.clone());
        best_asset.confidence = Some(score as u32);
        best_asset.match_reason = Some(final_reason.clone());

        return Ok(Some(ResolutionResult {
            query: raw_query.to_string(),
            resolved_entity: display_entity,
            entity_type: "brand".to_string(),
            asset_type,
            variant,
            selected_asset: best_asset.original_name.clone(),
            asset: best_asset,
            confidence: score as u32,
            match_confidence: conf,
            match_reason: final_reason,
            is_curated: false,
            available_variants: vec![variant],
        }));
    }

    Ok(None)
}

/// Generate smart suggestion hints when multiple variants exist or query is ambiguous.
pub fn suggest_variants(res: &ResolutionResult) -> Option<String> {
    if res.variant == AssetVariant::Default && res.available_variants.len() > 1 {
        let mut lines = Vec::new();
        lines.push(format!(
            "Multiple asset variants available for \"{}\":",
            res.resolved_entity
        ));
        lines.push("Try:".to_string());
        for v in &res.available_variants {
            if *v != AssetVariant::Default {
                lines.push(format!(
                    "  svgfetch {} --variant {}",
                    brands::slugify(&res.resolved_entity),
                    v.as_str()
                ));
            }
        }
        Some(lines.join("\n"))
    } else {
        None
    }
}

/// Clean unknown query error message with suggestions.
pub fn unknown_query_message(query: &str) -> String {
    format!(
        "No high-confidence SVG match found for \"{query}\".\n\n\
        Try a more specific query or use:\n\n  \
        svgfetch info {query}\n\n\
        Examples:\n\n  \
        svgfetch instagram\n  \
        svgfetch docker --variant icon\n  \
        svgfetch linux --variant mascot\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_natural_language_variants() {
        let (entity, variant) = parse_query_and_variant("instagram wordmark", None, false);
        assert_eq!(entity, "instagram");
        assert_eq!(variant, AssetVariant::Wordmark);

        let (entity, variant) = parse_query_and_variant("linux mascot", None, false);
        assert_eq!(entity, "linux");
        assert_eq!(variant, AssetVariant::Mascot);

        let (entity, variant) = parse_query_and_variant("docker full", None, false);
        assert_eq!(entity, "docker");
        assert_eq!(variant, AssetVariant::Full);

        let (entity, variant) = parse_query_and_variant("instagram icon", None, false);
        assert_eq!(entity, "instagram");
        assert_eq!(variant, AssetVariant::Icon);

        let (entity, variant) = parse_query_and_variant("kali linux mascot", None, false);
        assert_eq!(entity, "kali linux");
        assert_eq!(variant, AssetVariant::Mascot);
    }

    #[test]
    fn explicit_flag_takes_priority() {
        let (entity, variant) =
            parse_query_and_variant("instagram wordmark", Some(AssetVariant::Icon), false);
        assert_eq!(entity, "instagram");
        assert_eq!(variant, AssetVariant::Icon);
    }

    #[test]
    fn single_word_entity_matching_variant_name_is_preserved() {
        let (entity, variant) = parse_query_and_variant("icon", None, false);
        assert_eq!(entity, "icon");
        assert_eq!(variant, AssetVariant::Default);
    }

    #[test]
    fn semantic_scoring_distinguishes_entities_and_variants() {
        let asset = Asset {
            original_name: "Instagram wordmark 2016.svg".to_string(),
            mime: Some("image/svg+xml".to_string()),
            ..Default::default()
        };

        let (score_wm, conf_wm, _) =
            score_asset_variant_match("instagram", AssetVariant::Wordmark, &asset);
        assert!(score_wm >= 75);
        assert_eq!(conf_wm, MatchConfidence::High);

        let (score_icon, _, _) = score_asset_variant_match("instagram", AssetVariant::Icon, &asset);
        assert!(score_wm > score_icon);
    }

    #[test]
    fn facebook_heavily_penalizes_meta_asset() {
        let meta_asset = Asset {
            original_name: "Meta Platforms Inc. logo.svg".to_string(),
            mime: Some("image/svg+xml".to_string()),
            ..Default::default()
        };

        let (score, _, _) =
            score_asset_variant_match("facebook", AssetVariant::Wordmark, &meta_asset);
        assert_eq!(score, 0); // clamped to 0 due to -250 penalty
    }
}
