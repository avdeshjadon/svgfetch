//! Curated brand logo registry backed by `svgfetch-icons` CDN.
//!
//! Provides zero-latency brand matching from the bundled baseline catalog,
//! automatic live synchronization with `svgfetch-icons` CDN, and streams high-resolution
//! vector assets directly from the global jsDelivr edge CDN.

pub mod catalog;
pub mod contribute;
pub mod download;
pub mod remote;
pub mod sync;

pub use catalog::{
    find_curated_brand, get_curated_catalog, update_in_memory_catalog, CuratedEntry,
    CuratedVariants,
};
pub use contribute::{auto_contribute_brand, classify_brand_category, find_icons_repo_path};
pub use download::{download_curated_file, fetch_curated_bytes, CDN_BASE_URL, RAW_BASE_URL};
pub use remote::{report_remote_curation, trigger_remote_curation, RemoteCurationResult};
pub use sync::{
    auto_sync_if_needed, is_cache_expired, manifest_cache_path, sync_curated_catalog,
    MANIFEST_CACHE_TTL, MANIFEST_CDN_URL, MANIFEST_RAW_URL,
};

use crate::models::AssetVariant;
use std::io::{IsTerminal, Write};

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stderr(), crossterm::cursor::Show);
    }
}

#[derive(Debug, Clone)]
struct VariantOption {
    label: &'static str,
    filename: String,
    leaf: String,
}

/// Select which filename to download, prompting interactively with arrow keys and 'v' preview
/// if multiple variants exist.
pub async fn select_variant(
    entry: &CuratedEntry,
    variant_flag: Option<AssetVariant>,
    is_interactive: bool,
) -> Option<String> {
    let variants = entry.variants();

    // 1. If user explicitly specified variant flag, bypass interactive prompt
    if let Some(v) = variant_flag {
        match v {
            AssetVariant::Icon => {
                if let Some(icon) = &variants.icon_file {
                    return Some(icon.clone());
                } else {
                    eprintln!(
                        "[info] Icon mark not available for \"{}\". Downloading normal logo...",
                        entry.name
                    );
                    return Some(variants.full_file.unwrap_or_else(|| entry.files[0].clone()));
                }
            }
            AssetVariant::Full | AssetVariant::Default => {
                if let Some(full) = &variants.full_file {
                    return Some(full.clone());
                }
            }
            _ => {}
        }
    }

    // 2. Build list of available variant options
    let mut options: Vec<VariantOption> = Vec::new();
    for file in &entry.files {
        let leaf = leaf_name(file).to_string();
        let lower = leaf.to_lowercase();
        let label = if lower.contains("-icon") || lower.contains("_icon") {
            "Icon Mark"
        } else if lower.contains("-dark") || lower.contains("_dark") {
            "Dark Logo"
        } else if lower.contains("-white") || lower.contains("_white") {
            "White Logo"
        } else if lower.contains("-wordmark") || lower.contains("_wordmark") {
            "Wordmark"
        } else {
            "Full Logo"
        };
        options.push(VariantOption {
            label,
            filename: file.clone(),
            leaf,
        });
    }

    if options.is_empty() {
        return None;
    }

    // 3. If only 1 option or non-interactive terminal, return immediately
    if options.len() == 1 || !is_interactive || !std::io::stdin().is_terminal() {
        let chosen = variants
            .full_file
            .unwrap_or_else(|| options[0].filename.clone());
        let leaf = leaf_name(&chosen);
        eprintln!(
            "[curated] Found \"{}\" in svgfetch-icons library: {}",
            entry.name, leaf
        );
        return Some(chosen);
    }

    // 4. Interactive arrow-key navigation with 'v' preview
    prompt_interactive_selection(&entry.name, &options).await
}

async fn prompt_interactive_selection(
    brand_name: &str,
    options: &[VariantOption],
) -> Option<String> {
    use crossterm::cursor::{MoveToColumn, MoveUp};
    use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};

    eprintln!(
        "[curated] Found \"{}\" with {} variants.",
        brand_name,
        options.len()
    );
    eprintln!(
        "Use [↑/↓] to navigate, [v] to preview, [Enter] to select & download, [Esc/q] to cancel:\r"
    );

    let mut selected = 0;
    let _guard = RawModeGuard;
    if enable_raw_mode().is_err() {
        return Some(options[0].filename.clone());
    }
    let _ = crossterm::execute!(std::io::stderr(), crossterm::cursor::Hide);

    let draw_menu = |sel: usize| -> std::io::Result<()> {
        let mut err = std::io::stderr();
        for (i, opt) in options.iter().enumerate() {
            crossterm::queue!(err, Clear(ClearType::CurrentLine), MoveToColumn(0))?;
            if i == sel {
                write!(
                    err,
                    "  \x1b[1;36m>\x1b[0m \x1b[1m{}. {:<10}\x1b[0m ({})  \x1b[90m[Press 'v' to preview]\x1b[0m\r\n",
                    i + 1,
                    opt.label,
                    opt.leaf
                )?;
            } else {
                write!(
                    err,
                    "    {}. {:<10} ({})  \x1b[90m[Press 'v' to preview]\x1b[0m\r\n",
                    i + 1,
                    opt.label,
                    opt.leaf
                )?;
            }
        }
        err.flush()?;
        Ok(())
    };

    let _ = draw_menu(selected);

    loop {
        if let Ok(event) = event::read() {
            match event {
                Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind,
                    ..
                }) if kind == KeyEventKind::Press || kind == KeyEventKind::Repeat => {
                    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                        drop(_guard);
                        eprintln!("\r");
                        return None;
                    }

                    match code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            if selected > 0 {
                                selected -= 1;
                            } else {
                                selected = options.len() - 1;
                            }
                            let _ = crossterm::queue!(
                                std::io::stderr(),
                                MoveUp(options.len() as u16),
                                MoveToColumn(0)
                            );
                            let _ = draw_menu(selected);
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            if selected + 1 < options.len() {
                                selected += 1;
                            } else {
                                selected = 0;
                            }
                            let _ = crossterm::queue!(
                                std::io::stderr(),
                                MoveUp(options.len() as u16),
                                MoveToColumn(0)
                            );
                            let _ = draw_menu(selected);
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let idx = (c as usize) - ('1' as usize);
                            if idx < options.len() {
                                selected = idx;
                                let _ = crossterm::queue!(
                                    std::io::stderr(),
                                    MoveUp(options.len() as u16),
                                    MoveToColumn(0)
                                );
                                let _ = draw_menu(selected);
                            }
                        }
                        KeyCode::Char('v') | KeyCode::Char('V') => {
                            // Temporarily leave raw mode to print ANSI graphic & launch retina window
                            let _ = disable_raw_mode();
                            let _ = crossterm::execute!(std::io::stderr(), crossterm::cursor::Show);

                            let opt = &options[selected];
                            eprintln!(
                                "\r\n[preview] Fetching vector preview for {} ({})...",
                                brand_name, opt.leaf
                            );

                            if let Some(bytes) = fetch_curated_bytes(&opt.filename).await {
                                if let Some(ansi) =
                                    crate::ui::preview::render_halfblocks_to_ansi(&bytes, 42, 12)
                                {
                                    eprint!("{}", ansi);
                                }
                                let _ = crate::ui::viewer::open_vector_window(&opt.leaf, &bytes);
                                eprintln!(
                                    "(Vector window launched. Use [↑/↓] to navigate, [Enter] to download)\r\n"
                                );
                            } else {
                                eprintln!(
                                    "[warn] Could not load preview bytes for {}.\r\n",
                                    opt.leaf
                                );
                            }

                            let _ = enable_raw_mode();
                            let _ = crossterm::execute!(std::io::stderr(), crossterm::cursor::Hide);
                            let _ = draw_menu(selected);
                        }
                        KeyCode::Enter => {
                            drop(_guard);
                            eprintln!(
                                "\r\n[curated] Selected: {} ({})",
                                options[selected].label, options[selected].leaf
                            );
                            return Some(options[selected].filename.clone());
                        }
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            drop(_guard);
                            eprintln!("\r\n[info] Selection cancelled.");
                            return None;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
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

    #[tokio::test]
    async fn select_variant_respects_variant_flags_and_defaults() {
        let instagram = find_curated_brand("instagram").unwrap();
        // 1. With --variant icon flag
        let icon_choice = select_variant(&instagram, Some(AssetVariant::Icon), false).await;
        assert_eq!(icon_choice, Some("meta/instagram-icon.svg".to_string()));

        // 2. With --variant full flag
        let full_choice = select_variant(&instagram, Some(AssetVariant::Full), false).await;
        assert_eq!(full_choice, Some("meta/instagram.svg".to_string()));

        // 3. Non-interactive default selects primary full logo
        let default_choice = select_variant(&instagram, None, false).await;
        assert_eq!(default_choice, Some("meta/instagram.svg".to_string()));
    }
}
