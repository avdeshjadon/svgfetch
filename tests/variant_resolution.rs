//! Comprehensive regression test suite for exact entity and variant resolution.

use svgfetch::brands::BrandRegistry;
use svgfetch::models::AssetVariant;
use svgfetch::resolution::{
    parse_query_and_variant, score_asset_variant_match, unknown_query_message,
};

#[test]
fn test_query_variant_parsing_and_priority() {
    // 1. Natural language suffix
    let (entity, var) = parse_query_and_variant("instagram wordmark", None, false);
    assert_eq!(entity, "instagram");
    assert_eq!(var, AssetVariant::Wordmark);

    let (entity, var) = parse_query_and_variant("docker full", None, false);
    assert_eq!(entity, "docker");
    assert_eq!(var, AssetVariant::Full);

    let (entity, var) = parse_query_and_variant("linux mascot", None, false);
    assert_eq!(entity, "linux");
    assert_eq!(var, AssetVariant::Mascot);

    // 2. Multi-word variant
    let (entity, var) = parse_query_and_variant("docker text logo", None, false);
    assert_eq!(entity, "docker");
    assert_eq!(var, AssetVariant::Wordmark);

    // 3. Explicit flag priority over query words
    let (entity, var) =
        parse_query_and_variant("instagram wordmark", Some(AssetVariant::Icon), false);
    assert_eq!(entity, "instagram");
    assert_eq!(var, AssetVariant::Icon);

    let (entity, var) = parse_query_and_variant("linux mascot", Some(AssetVariant::Default), false);
    assert_eq!(entity, "linux");
    assert_eq!(var, AssetVariant::Default);
}

#[test]
fn test_entity_resolution_facebook_vs_meta() {
    let reg = BrandRegistry::load();

    // Facebook must NEVER resolve to Meta
    let fb = reg.resolve("facebook").expect("facebook");
    assert_eq!(fb.name, "Facebook");
    assert_ne!(fb.name, "Meta");
    assert!(fb.file.contains("Facebook"));
    assert_eq!(
        fb.file_for_variant(AssetVariant::Wordmark),
        Some("File:Facebook New Logo (2015).svg")
    );

    // Meta must resolve to Meta Platforms
    let meta = reg.resolve("meta").expect("meta");
    assert_eq!(meta.name, "Meta");
    assert_ne!(meta.name, "Facebook");
    assert!(meta.file.contains("Meta Platforms"));
}

#[test]
fn test_entity_resolution_linux_vs_tux() {
    let reg = BrandRegistry::load();

    // Linux default should be Linux branding, NOT Tux
    let linux = reg.resolve("linux").expect("linux");
    assert_eq!(linux.name, "Linux");
    let linux_default = linux.file_for_variant(AssetVariant::Default).unwrap();
    assert!(!linux_default.eq("File:Tux.svg"));
    assert_eq!(
        linux.file_for_variant(AssetVariant::Mascot),
        Some("File:Tux.svg")
    );

    // Tux must resolve to Tux
    let tux = reg.resolve("tux").expect("tux");
    assert_eq!(tux.name, "Tux");
    assert_eq!(
        tux.file_for_variant(AssetVariant::Default),
        Some("File:Tux.svg")
    );
    assert_eq!(
        tux.file_for_variant(AssetVariant::Mascot),
        Some("File:Tux.svg")
    );
}

#[test]
fn test_entity_resolution_kali_linux() {
    let reg = BrandRegistry::load();

    let kali = reg.resolve("kali linux").expect("kali linux");
    assert_eq!(kali.name, "Kali Linux");
    assert_eq!(
        kali.file_for_variant(AssetVariant::Mascot),
        Some("File:Kali-dragon-icon.svg")
    );
}

#[test]
fn test_entity_resolution_docker_variants() {
    let reg = BrandRegistry::load();

    let docker = reg.resolve("docker").expect("docker");
    assert_eq!(docker.name, "Docker");
    assert_eq!(
        docker.file_for_variant(AssetVariant::Icon),
        Some("File:Docker (container engine) logo.svg")
    );
    assert_eq!(
        docker.file_for_variant(AssetVariant::Full),
        Some("File:Docker Logo.svg")
    );
}

#[test]
fn test_entity_resolution_github_variants() {
    let reg = BrandRegistry::load();

    let gh = reg.resolve("github").expect("github");
    assert_eq!(gh.name, "GitHub");
    assert_eq!(
        gh.file_for_variant(AssetVariant::Icon),
        Some("File:Octicons-mark-github.svg")
    );
    assert_eq!(
        gh.file_for_variant(AssetVariant::Wordmark),
        Some("File:GitHub logo 2013.svg")
    );
}

#[test]
fn test_entity_resolution_python_variants() {
    let reg = BrandRegistry::load();

    let python = reg.resolve("python").expect("python");
    assert_eq!(python.name, "Python");
    assert_eq!(
        python.file_for_variant(AssetVariant::Icon),
        Some("File:Python-logo-notext.svg")
    );
    assert_eq!(
        python.file_for_variant(AssetVariant::Wordmark),
        Some("File:Python logo and wordmark.svg")
    );
}

#[test]
fn test_semantic_ranking_penalties() {
    // Entity mismatch: Facebook querying Meta asset
    let meta_asset = svgfetch::models::Asset {
        original_name: "Meta Platforms Inc. logo.svg".into(),
        mime: Some("image/svg+xml".into()),
        ..Default::default()
    };
    let (score_fb, conf_fb, _) =
        score_asset_variant_match("facebook", AssetVariant::Default, &meta_asset);
    assert_eq!(score_fb, 0);
    assert_eq!(conf_fb, svgfetch::search::MatchConfidence::Low);

    // Entity mismatch: GitHub querying GitLab asset
    let gitlab_asset = svgfetch::models::Asset {
        original_name: "GitLab Logo.svg".into(),
        mime: Some("image/svg+xml".into()),
        ..Default::default()
    };
    let (score_gh, conf_gh, _) =
        score_asset_variant_match("github", AssetVariant::Default, &gitlab_asset);
    assert_eq!(score_gh, 0);
    assert_eq!(conf_gh, svgfetch::search::MatchConfidence::Low);

    // Variant mismatch: icon requested on wordmark asset
    let wm_asset = svgfetch::models::Asset {
        original_name: "Docker wordmark logotype.svg".into(),
        mime: Some("image/svg+xml".into()),
        ..Default::default()
    };
    let (score_icon, _, _) = score_asset_variant_match("docker", AssetVariant::Icon, &wm_asset);
    let (score_wm, _, _) = score_asset_variant_match("docker", AssetVariant::Wordmark, &wm_asset);
    assert!(score_wm > score_icon);
}

#[test]
fn test_unknown_query_message_format() {
    let msg = unknown_query_message("my_fake_query");
    assert!(msg.contains("No high-confidence SVG match found for \"my_fake_query\"."));
    assert!(msg.contains("svgfetch info my_fake_query"));
    assert!(msg.contains("svgfetch instagram"));
}
