//! `svgfetch uninstall` — completely remove svgfetch from the system.
//!
//! Deletes everything the app owns so a fresh install behaves like a brand
//! new install:
//! * the config directory (config.toml + cache + recent searches),
//! * the default download folder (`~/Downloads/svgfetch`),
//! * the installed `svgfetch` binaries (including the running one).

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cli::ui::{self, LogLine, Tally, Ui};
use crate::config::Settings;
use crate::error::{Error, Result};

/// The download folder is only wiped automatically when it is the default
/// `svgfetch` folder. A custom `download_directory` may point anywhere, so it
/// is left alone (and reported) instead of risking unrelated user data.
const DEFAULT_DOWNLOAD_NAME: &str = "svgfetch";

/// What would be removed, computed up front so we can show it and confirm.
#[derive(Debug, Clone)]
pub struct UninstallPlan {
    pub config_dir: Option<PathBuf>,
    pub download_dir: Option<PathBuf>,
    pub binaries: Vec<PathBuf>,
}

impl UninstallPlan {
    /// Everything about to be deleted, oldest first (config → downloads → bins).
    pub fn targets(&self) -> Vec<&Path> {
        let mut out = Vec::new();
        if let Some(p) = &self.config_dir {
            out.push(p.as_path());
        }
        if let Some(p) = &self.download_dir {
            out.push(p.as_path());
        }
        out.extend(self.binaries.iter().map(PathBuf::as_path));
        out
    }
}

/// Derive the plan (no filesystem writes).
pub fn collect_plan(settings: &Settings) -> UninstallPlan {
    let mut plan = collect_plan_with_exe(settings, std::env::current_exe().ok().as_deref());
    let ext = if cfg!(windows) { ".exe" } else { "" };

    let mut extra_dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        if local_bin.exists() {
            extra_dirs.push(local_bin);
        }
        let cargo_bin = home.join(".cargo").join("bin");
        if cargo_bin.exists() {
            extra_dirs.push(cargo_bin);
        }
    }
    let usr_local = PathBuf::from("/usr/local/bin");
    if usr_local.exists() {
        extra_dirs.push(usr_local);
    }

    for dir in extra_dirs {
        for name in ["svg", "svgfetch", "svg-fetch", "get-svg", "getsvg"] {
            let path = dir.join(format!("{name}{ext}"));
            if path.exists() && !plan.binaries.contains(&path) {
                plan.binaries.push(path);
            }
            let stale = dir.join(format!("{name}{ext}.old"));
            if stale.exists() && !plan.binaries.contains(&stale) {
                plan.binaries.push(stale);
            }
        }
    }

    plan
}

/// Same as [`collect_plan`], but the running binary is supplied explicitly so
/// tests can pin down binary discovery without depending on the ambient test
/// environment (which varies across platforms and toolchains).
fn collect_plan_with_exe(settings: &Settings, exe: Option<&Path>) -> UninstallPlan {
    let ext = if cfg!(windows) { ".exe" } else { "" };

    let config_dir = settings.config_dir.clone();
    let config_dir = (config_dir.exists()).then_some(config_dir);

    let download_dir = (settings.download_dir.exists()
        && settings.download_dir != settings.config_dir
        && settings
            .download_dir
            .file_name()
            .map(|n| n == DEFAULT_DOWNLOAD_NAME || n == "get-svg")
            .unwrap_or(false))
    .then_some(settings.download_dir.clone());

    let mut binaries = Vec::new();
    if let Some(exe) = exe {
        if let Some(dir) = exe.parent() {
            for name in ["svg", "svgfetch", "svg-fetch", "get-svg", "getsvg"] {
                let path = dir.join(format!("{name}{ext}"));
                if path.exists() {
                    binaries.push(path);
                }
                let stale = dir.join(format!("{name}{ext}.old"));
                if stale.exists() {
                    binaries.push(stale);
                }
            }
        }
    }

    UninstallPlan {
        config_dir,
        download_dir,
        binaries,
    }
}

/// Run the full uninstall. Returns a process exit code.
pub fn run_uninstall(yes: bool) -> Result<i32> {
    let ui = Ui::detect();
    let settings = Settings::load().unwrap_or_else(|_| Settings::default());
    let plan = ui.think("Scanning system for svgfetch files\u{2026}", || {
        collect_plan(&settings)
    });

    if plan.targets().is_empty() {
        ui.print_box(&[
            "svgfetch uninstall".to_string(),
            String::new(),
            "Nothing to remove \u{2014} svgfetch already leaves no trace on this system."
                .to_string(),
        ]);
        return Ok(0);
    }

    ui.print_box(&["svgfetch uninstall  \u{00b7}  Purge tool, caches & binaries".to_string()]);
    println!();
    println!("  Targets identified for removal:");
    if let Some(p) = &plan.config_dir {
        print_target("config", p, "Settings, cache, and search index");
    }
    if let Some(p) = &plan.download_dir {
        print_target("downloads", p, "SVGs downloaded by svgfetch");
    }
    for b in &plan.binaries {
        print_target("binary", b, "Executable binary");
    }
    if plan.download_dir.is_none() && settings.download_dir.exists() {
        println!();
        println!(
            "  {} Custom download directory left untouched (not named \"svgfetch\"):",
            ui::yellow("!", ui.color)
        );
        println!("         {}", ui::tilde(&settings.download_dir));
        println!("         Delete it manually if you also want those files removed.");
    }
    println!();

    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(Error::Other(
                "uninstall requires confirmation but stdin is not a TTY; pass --yes to proceed."
                    .into(),
            ));
        }
        eprint!("  Are you sure? This permanently deletes the files above. [y/N]: ");
        std::io::stderr().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            println!(
                "  {} Uninstall cancelled.",
                ui::yellow("[cancelled]", ui.color)
            );
            return Ok(0);
        }
    }
    println!();

    let mut tally = Tally::default();

    // 1. Purge config & persistent caches
    let lines = ui.step("Configuration & caches", 1, 4, || {
        std::thread::sleep(Duration::from_millis(150));
        remove_dir_report(&plan.config_dir, "Configuration & cache")
    });
    tally.add(&lines);

    // 2. Purge downloads
    let lines = ui.step("Downloaded assets", 2, 4, || {
        std::thread::sleep(Duration::from_millis(150));
        remove_dir_report(&plan.download_dir, "Downloaded assets")
    });
    tally.add(&lines);

    // 3. Unregister npm global package
    let lines = ui.step("Global npm package", 3, 4, || {
        std::thread::sleep(Duration::from_millis(120));
        if let Ok(output) = std::process::Command::new("npm")
            .args(["uninstall", "-g", "svgfetch", "@avdeshjadon/get-svg"])
            .output()
        {
            if output.status.success() {
                return vec![LogLine::ok("unlinked", "Global npm package unregistered")];
            }
        }
        Vec::new()
    });
    tally.add(&lines);

    // 4. Unlink binaries
    let lines = ui.step("Executable binaries", 4, 4, || {
        std::thread::sleep(Duration::from_millis(150));
        let mut out = Vec::new();
        if let Ok(cur) = std::env::current_exe() {
            for bin in &plan.binaries {
                if bin == &cur {
                    if let Some(line) = remove_running_binary(bin) {
                        out.push(line);
                    }
                } else {
                    let removed = std::fs::remove_file(bin).is_ok();
                    if removed {
                        out.push(unlinked(bin));
                    } else {
                        out.push(LogLine::skipped(ui::tilde(bin)));
                    }
                }
            }
        } else {
            for bin in &plan.binaries {
                let removed = std::fs::remove_file(bin).is_ok();
                if removed {
                    out.push(unlinked(bin));
                } else {
                    out.push(LogLine::skipped(ui::tilde(bin)));
                }
            }
        }
        out
    });
    tally.add(&lines);

    println!();
    ui.print_summary(&tally);

    let headline = if tally.warnings > 0 {
        format!(
            "svgfetch was removed, but {} warning(s) need review.",
            tally.warnings
        )
    } else {
        "svgfetch has been completely removed from this system.".to_string()
    };
    ui.print_box(&[
        headline,
        String::new(),
        "Thank you for using svgfetch! Until next time!".to_string(),
        String::new(),
        "Reinstall anytime with:".to_string(),
        "  npm install -g svgfetch".to_string(),
        "  curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | sh"
            .to_string(),
    ]);
    println!();
    Ok(0)
}

fn unlinked(path: &Path) -> LogLine {
    LogLine::ok("unlinked", ui::tilde(path))
}

fn print_target(label: &str, path: &Path, what: &str) {
    println!(
        "  \u{2022} {:<10} {} \u{2013} {what}",
        label,
        ui::tilde(path)
    );
}

fn remove_dir_report(dir: &Option<PathBuf>, label: &str) -> Vec<LogLine> {
    match dir {
        None => vec![LogLine::skipped(format!("{label} already removed"))],
        Some(p) => match std::fs::remove_dir_all(p) {
            Ok(()) => vec![LogLine::ok(
                "deleted",
                format!("{label} removed  ({})", ui::tilde(p)),
            )],
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                vec![LogLine::skipped(format!("{label} already removed"))]
            }
            Err(e) => vec![LogLine::notice(format!(
                "Could not remove {label} ({}): {e}",
                ui::tilde(p)
            ))],
        },
    }
}

/// The running binary cannot be deleted while executing on Windows, so we
/// schedule a detached, delayed delete there. On Unix unlinking the file is
/// safe — the inode lives until this process exits.
fn remove_running_binary(path: &Path) -> Option<LogLine> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::fs::remove_file(path).or_else(|_| {
            std::process::Command::new("cmd")
                .creation_flags(0x0800_0000 | 0x0000_0008) // CREATE_NO_WINDOW | DETACHED_PROCESS
                .args([
                    "/C",
                    "ping -n 3 127.0.0.1 >nul & del /q",
                    &path.to_string_lossy(),
                ])
                .spawn()
                .map(|_| ())
        });
        None
    }
    #[cfg(not(windows))]
    {
        let removed = std::fs::remove_file(path).is_ok();
        Some(if removed {
            LogLine::ok("unlinked", ui::tilde(path))
        } else {
            LogLine::notice(format!("{} (running binary)", ui::tilde(path)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(config_dir: &Path, download_dir: &Path) -> Settings {
        Settings {
            config_dir: config_dir.to_path_buf(),
            download_dir: download_dir.to_path_buf(),
            ..Settings::default()
        }
    }

    #[test]
    fn default_folders_are_in_plan() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("svgfetch");
        let dl = dir.path().join("svgfetch");
        std::fs::create_dir_all(&cfg).unwrap();
        let plan = collect_plan_with_exe(&settings_with(&cfg, &dl), None);
        assert!(plan.config_dir.is_some());
        assert!(plan.download_dir.is_none()); // same path as config dir -> deduped
    }

    #[test]
    fn distinct_svgfetch_downloads_included() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("cfg").join("svgfetch");
        let dl = dir.path().join("dl").join("svgfetch");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&dl).unwrap();
        let plan = collect_plan_with_exe(&settings_with(&cfg, &dl), None);
        assert!(plan.config_dir.is_some());
        assert!(plan.download_dir.is_some());
    }

    #[test]
    fn custom_download_dir_is_never_wiped() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("svgfetch");
        let custom = dir.path().join("my-icons");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&custom).unwrap();
        let plan = collect_plan_with_exe(&settings_with(&cfg, &custom), None);
        assert!(plan.download_dir.is_none());
    }

    #[test]
    fn no_trace_yields_empty_plan() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("svgfetch");
        let plan = collect_plan_with_exe(&settings_with(&cfg, &cfg), None);
        assert!(plan.targets().is_empty());
    }

    #[test]
    fn running_binary_and_siblings_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let bindir = dir.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        let ext = if cfg!(windows) { ".exe" } else { "" };
        for name in ["svgfetch", "svg-fetch"] {
            std::fs::write(bindir.join(format!("{name}{ext}")), b"").unwrap();
        }
        let exe = bindir.join(format!("svgfetch{ext}"));
        let plan = collect_plan_with_exe(&settings_with(&bindir, &bindir), Some(&exe));
        let names: Vec<String> = plan
            .binaries
            .iter()
            .map(|b| b.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec![format!("svgfetch{ext}"), format!("svg-fetch{ext}")]
        );
    }

    #[test]
    fn plan_targets_are_lowercased_safe() {
        let plan = UninstallPlan {
            config_dir: Some(PathBuf::from("/x/svgfetch")),
            download_dir: None,
            binaries: vec![PathBuf::from("/x/bin/svgfetch")],
        };
        let targets = plan.targets();
        assert!(targets
            .iter()
            .all(|p| p.to_string_lossy().contains("svgfetch")));
    }
}
