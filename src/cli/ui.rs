//! Shared terminal presentation for the install / update / uninstall flows.
//!
//! This module is presentational only: it never changes what a command does,
//! only how it looks. Rendering degrades to plain, ANSI-free lines whenever
//! stdout/stderr is not a TTY, when `NO_COLOR` is set, or when `TERM=dumb`, so
//! piped / CI usage stays machine-friendly.

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Accent colour (256-colour). Re-theme every flow by changing this one line.
pub const ACCENT: u8 = 209;

/// Animation frame period.
pub const TICK: Duration = Duration::from_millis(70);

/// Minimum time a step's animation stays on screen so it is actually readable.
pub const MIN_STEP: Duration = Duration::from_millis(450);

/// Glyphs pulsed by the "thinking" line (ping-pong order).
const GLYPHS: [char; 10] = ['·', '✢', '✳', '✶', '✻', '✽', '✻', '✶', '✳', '✢'];
const SCANNER_CELLS: usize = 10;
const SHIMMER_PAD: usize = 3;
const TAG_WIDTH: usize = 9;

fn accent_sgr() -> String {
    format!("1;38;5;{ACCENT}")
}

const CURSOR_HIDE: &[u8] = b"\x1b[?25l";
/// Clear the current line and restore the cursor.
const CURSOR_RESTORE: &[u8] = b"\r\x1b[2K\x1b[?25h";

fn write_stderr(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

/// Install a panic hook once so `panic = "abort"` builds still restore the
/// cursor, even though unwinding (and `Drop`) never runs.
fn install_panic_guard() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            write_stderr(CURSOR_RESTORE);
            previous(info);
        }));
    });
}

/// Restore the cursor if the process is interrupted from the terminal. The
/// closure only uses async-signal-safe calls, then exits with the shell's
/// conventional `128 + signum` status.
#[cfg(unix)]
fn install_signal_guard() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        extern "C" fn restore_and_exit(sig: libc::c_int) {
            unsafe {
                let _ = libc::write(2, CURSOR_RESTORE.as_ptr().cast(), CURSOR_RESTORE.len());
                libc::_exit(128 + sig);
            }
        }
        libc::signal(
            libc::SIGINT,
            restore_and_exit as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            restore_and_exit as *const () as libc::sighandler_t,
        );
    });
}

#[cfg(not(unix))]
fn install_signal_guard() {}

fn install_terminal_guards() {
    install_panic_guard();
    install_signal_guard();
}

/// Hides the cursor while a live animation is on screen and unconditionally
/// shows it again when dropped (normal return, panic, or signal).
struct CursorGuard;

impl CursorGuard {
    fn hide() -> CursorGuard {
        install_terminal_guards();
        write_stderr(CURSOR_HIDE);
        CursorGuard
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        write_stderr(CURSOR_RESTORE);
    }
}

/// Wrap `text` in an SGR sequence when colour is enabled.
pub fn paint(text: &str, sgr: &str, color: bool) -> String {
    if color {
        format!("\x1b[{sgr}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn accent(text: &str, color: bool) -> String {
    paint(text, &accent_sgr(), color)
}
pub fn green(text: &str, color: bool) -> String {
    paint(text, "32", color)
}
pub fn yellow(text: &str, color: bool) -> String {
    paint(text, "33", color)
}
pub fn red(text: &str, color: bool) -> String {
    paint(text, "31", color)
}
pub fn dim(text: &str, color: bool) -> String {
    paint(text, "2", color)
}

/// Whether colour and animation should be used for this process.
#[derive(Debug, Clone, Copy)]
pub struct Ui {
    pub color: bool,
    pub animate: bool,
}

impl Ui {
    /// Detect terminal capabilities from the environment.
    pub fn detect() -> Ui {
        let no_color = std::env::var_os("NO_COLOR").is_some();
        let dumb = std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false);
        let out_tty = std::io::stdout().is_terminal();
        let err_tty = std::io::stderr().is_terminal();
        let enabled = !no_color && !dumb;
        Ui {
            color: enabled && out_tty,
            animate: enabled && out_tty && err_tty,
        }
    }

    /// Force a plain, non-animated UI (tests / captured output).
    pub fn plain() -> Ui {
        Ui {
            color: false,
            animate: false,
        }
    }

    /// Start an animated "thinking" line, or `None` when animation is off.
    pub fn spinner(&self, label: &str) -> Option<Spinner> {
        Spinner::start(self.animate, label)
    }

    /// Run `f` under a spinner without rendering a completed step. Used for the
    /// initial scan/search phases where only a single line of output follows.
    pub fn think<F, R>(&self, label: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let spinner = self.spinner(label);
        let out = f();
        if let Some(s) = spinner {
            let _ = s.finish_min(MIN_STEP);
        }
        out
    }

    /// Run `f`, animate while it works, then print the completed step tree.
    pub fn step<F>(&self, label: &str, index: usize, total: usize, f: F) -> Vec<LogLine>
    where
        F: FnOnce() -> Vec<LogLine>,
    {
        let spinner = self.spinner(label);
        let lines = f();
        let elapsed = match spinner {
            Some(s) => s.finish_min(MIN_STEP),
            None => Duration::ZERO,
        };
        self.render_step(label, index, total, elapsed, &lines);
        lines
    }

    /// Print a completed step header followed by its child log lines.
    pub fn render_step(
        &self,
        label: &str,
        index: usize,
        total: usize,
        elapsed: Duration,
        lines: &[LogLine],
    ) {
        let warned = lines.iter().any(|l| l.level.is_warning());
        let dot = if warned {
            yellow("●", self.color)
        } else {
            green("●", self.color)
        };
        if self.animate {
            println!(
                "  {dot} {label} [{index}/{total}]  {}",
                dim(&fmt_secs(elapsed), self.color)
            );
        } else {
            println!("  {dot} {label} [{index}/{total}]");
        }
        for (i, line) in lines.iter().enumerate() {
            let branch = if i == 0 { "    ⎿ " } else { "      " };
            println!("{branch}{}", line.render(self.color));
        }
        println!();
    }

    /// A clean one-line success message when nothing needs to be done.
    pub fn up_to_date(&self, version: &str) {
        println!(
            "  {} svgfetch v{version} is already up to date.",
            green("✓", self.color)
        );
    }

    /// A multi-line informational notice (yellow `!`).
    pub fn notice(&self, lines: &[String]) {
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                println!("  {} {line}", yellow("!", self.color));
            } else {
                println!("    {line}");
            }
        }
        println!();
    }

    /// Print the `✓ done – skipped ! warnings` summary line.
    pub fn print_summary(&self, tally: &Tally) {
        println!("  {}", tally.summary(self.color));
        println!();
    }

    /// Print a bordered panel whose width tracks the longest line.
    pub fn print_box(&self, lines: &[String]) {
        println!("{}", boxed(lines, self.color));
    }
}

/// Format a duration as `1.2s`.
pub fn fmt_secs(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}

/// Shorten a path under the user's home directory to `~/…`.
pub fn tilde(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = path.strip_prefix(&home) {
            let rest = rest.to_string_lossy();
            if rest.is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest);
        }
    }
    path.display().to_string()
}

/// Severity of a completed log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    Skip,
    Warn,
    Error,
}

impl Level {
    pub fn is_warning(self) -> bool {
        matches!(self, Level::Warn | Level::Error)
    }
}

/// One `✓ deleted  path` style child line in a step tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub level: Level,
    pub tag: &'static str,
    pub text: String,
}

impl LogLine {
    pub fn new(level: Level, tag: &'static str, text: impl Into<String>) -> Self {
        LogLine {
            level,
            tag,
            text: text.into(),
        }
    }

    pub fn ok(tag: &'static str, text: impl Into<String>) -> Self {
        Self::new(Level::Ok, tag, text)
    }

    pub fn skipped(text: impl Into<String>) -> Self {
        Self::new(Level::Skip, "skipped", text)
    }

    pub fn notice(text: impl Into<String>) -> Self {
        Self::new(Level::Warn, "notice", text)
    }

    pub fn failed(text: impl Into<String>) -> Self {
        Self::new(Level::Error, "failed", text)
    }

    /// Render as `✓ tag       text` with the tag coloured by level.
    pub fn render(&self, color: bool) -> String {
        let (sym, sgr) = match self.level {
            Level::Ok => ("✓", "32"),
            Level::Skip => ("–", "2"),
            Level::Warn => ("!", "33"),
            Level::Error => ("!", "31"),
        };
        let padded = format!("{sym} {:<width$}", self.tag, width = TAG_WIDTH);
        format!("{} {}", paint(&padded, sgr, color), self.text)
    }
}

/// Counts of completed / skipped / warning log lines across a flow.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub done: usize,
    pub skipped: usize,
    pub warnings: usize,
}

impl Tally {
    pub fn add(&mut self, lines: &[LogLine]) {
        for line in lines {
            match line.level {
                Level::Ok => self.done += 1,
                Level::Skip => self.skipped += 1,
                Level::Warn | Level::Error => self.warnings += 1,
            }
        }
    }

    pub fn summary(&self, color: bool) -> String {
        let warnings = if self.warnings > 0 {
            yellow(&format!("! {} warning(s)", self.warnings), color)
        } else {
            dim(&format!("! {} warnings", self.warnings), color)
        };
        format!(
            "{}  {}  {}",
            green(&format!("✓ {} done", self.done), color),
            dim(&format!("– {} skipped", self.skipped), color),
            warnings
        )
    }
}

/// Render a fixed-width, accent-bordered panel. Width is computed from the
/// longest line so nothing overflows (even the long curl reinstall command).
pub fn boxed(lines: &[String], color: bool) -> String {
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let inner = width + 4;
    let bar = "─".repeat(inner);
    let border = |text: &str| paint(text, &accent_sgr(), color);
    let mut out = String::new();
    out.push_str(&format!(
        "  {}{}{}\n",
        border("┌"),
        border(&bar),
        border("┐")
    ));
    for line in lines {
        let pad = width.saturating_sub(line.chars().count());
        out.push_str(&format!(
            "  {}  {}{}  {}\n",
            border("│"),
            line,
            " ".repeat(pad),
            border("│")
        ));
    }
    out.push_str(&format!("  {}{}{}", border("└"), border(&bar), border("┘")));
    out
}

/// Build one "thinking" frame: `✻ Shimmering label…  ■■▪▪▪▪▪▪▪▪  1.2s`.
pub fn thinking_frame(tick: usize, label: &str, elapsed: Duration, color: bool) -> String {
    let glyph = GLYPHS[(tick / 2) % GLYPHS.len()];
    if color {
        format!(
            "  {} {}  {}  {}",
            accent(&glyph.to_string(), true),
            shimmer(label, tick),
            scanner(tick),
            dim(&fmt_secs(elapsed), true)
        )
    } else {
        format!("  {glyph} {label}  {}", fmt_secs(elapsed))
    }
}

/// Sweep a bright highlight left to right across the label.
fn shimmer(label: &str, tick: usize) -> String {
    let chars: Vec<char> = label.chars().collect();
    let span = chars.len() + SHIMMER_PAD * 2;
    let head = (tick % span) as isize - SHIMMER_PAD as isize;
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let dist = (i as isize - head).abs();
        let sgr = match dist {
            0 => "1;38;5;255",
            1 => "38;5;252",
            2 => "38;5;248",
            _ => "38;5;244",
        };
        out.push_str(&paint(&ch.to_string(), sgr, true));
    }
    out
}

/// Ten-cell block scanner bouncing back and forth with a fading tail.
fn scanner(tick: usize) -> String {
    let n = SCANNER_CELLS;
    let period = 2 * (n - 1);
    let p = tick % period;
    let (pos, dir) = if p < n {
        (p, 1isize)
    } else {
        (period - p, -1isize)
    };
    let mut out = String::new();
    for i in 0..n {
        let sgr = if i == pos {
            accent_sgr()
        } else if i as isize == pos as isize - dir {
            "38;5;173".to_string()
        } else if i as isize == pos as isize - 2 * dir {
            "38;5;130".to_string()
        } else {
            "38;5;238".to_string()
        };
        let cell = if i == pos { "■" } else { "▪" };
        out.push_str(&paint(cell, &sgr, true));
    }
    out
}

/// A download progress bar frame:
/// `⬇ svgfetch v1.2.3  ▰▰▰▰▰▰▱▱▱▱  62%  3.1 MB/s  eta 2s`.
pub fn download_frame(
    label: &str,
    written: u64,
    total: u64,
    speed_bps: f64,
    eta_secs: f64,
    color: bool,
) -> String {
    let pct = written
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(0)
        .min(100);
    let filled = ((pct as usize * SCANNER_CELLS) / 100).min(SCANNER_CELLS);
    let bar = format!(
        "{}{}",
        paint(&"▰".repeat(filled), &accent_sgr(), color),
        dim(&"▱".repeat(SCANNER_CELLS - filled), color)
    );
    let speed = format!(
        "{}/s",
        crate::models::format_size(speed_bps.max(0.0) as u64)
    );
    let eta = if eta_secs >= 1.0 {
        format!("eta {}s", eta_secs.ceil() as u64)
    } else {
        "eta <1s".to_string()
    };
    format!(
        "  {} {label}  {bar}  {pct}%  {}  {}",
        accent("⬇", color),
        dim(&speed, color),
        dim(&eta, color)
    )
}

/// Live download progress, throttled to one `stderr` write per frame.
pub struct Progress {
    total: u64,
    written: u64,
    start: Instant,
    last: Instant,
    label: String,
    active: bool,
    _cursor: Option<CursorGuard>,
}

impl Progress {
    pub fn new(ui: &Ui, label: &str, total: u64) -> Progress {
        Progress {
            total,
            written: 0,
            start: Instant::now(),
            last: Instant::now(),
            label: label.to_string(),
            active: ui.animate,
            _cursor: ui.animate.then(CursorGuard::hide),
        }
    }

    pub fn update(&mut self, written: u64) {
        self.written = written;
        if !self.active || self.last.elapsed() < TICK {
            return;
        }
        self.last = Instant::now();
        let frame = self.frame();
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K{frame}");
        let _ = err.flush();
    }

    fn frame(&self) -> String {
        let secs = self.start.elapsed().as_secs_f64().max(0.001);
        let speed = self.written as f64 / secs;
        let eta = if speed > 0.0 && self.total > self.written {
            (self.total - self.written) as f64 / speed
        } else {
            0.0
        };
        download_frame(&self.label, self.written, self.total, speed, eta, true)
    }

    pub fn finish(&self) {
        if !self.active {
            return;
        }
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K");
        let _ = err.flush();
    }
}

/// A stop-on-drop animation thread. Dropping the guard always stops and joins
/// the thread and clears the line, so a panic or early return can never leave
/// a broken terminal.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    start: Instant,
    _cursor: CursorGuard,
}

impl Spinner {
    fn start(animate: bool, label: &str) -> Option<Spinner> {
        if !animate {
            return None;
        }
        let cursor = CursorGuard::hide();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let label = label.to_string();
        let handle = std::thread::spawn(move || {
            let start = Instant::now();
            let mut tick = 0usize;
            while !flag.load(Ordering::Relaxed) {
                let frame = thinking_frame(tick, &label, start.elapsed(), true);
                let mut err = std::io::stderr();
                let _ = write!(err, "\r\x1b[2K{frame}");
                let _ = err.flush();
                std::thread::sleep(TICK);
                tick += 1;
            }
        });
        Some(Spinner {
            stop,
            handle: Some(handle),
            start: Instant::now(),
            _cursor: cursor,
        })
    }

    /// Stop the animation, first holding the frame for at least `min` so it is
    /// readable. Returns the total elapsed time.
    pub fn finish_min(mut self, min: Duration) -> Duration {
        let elapsed = self.start.elapsed();
        if elapsed < min {
            std::thread::sleep(min - elapsed);
        }
        let total = self.start.elapsed();
        self.stop_inner();
        total
    }

    fn stop_inner(&mut self) {
        if self.handle.is_none() {
            return;
        }
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let mut err = std::io::stderr();
        let _ = write!(err, "\r\x1b[2K");
        let _ = err.flush();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_width_tracks_longest_line() {
        let lines = vec!["hi".to_string(), "a much longer line".to_string()];
        let rendered = boxed(&lines, false);
        let longest = "a much longer line".chars().count() + 4 + 2 + 2;
        for line in rendered.lines() {
            assert_eq!(line.chars().count(), longest, "ragged box: {line:?}");
        }
        assert!(!rendered.contains('\u{1b}'), "ANSI leaked with colour off");
    }

    #[test]
    fn box_handles_long_curl_command_without_overflow() {
        let cmd = "  curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | sh".to_string();
        let rendered = boxed(std::slice::from_ref(&cmd), false);
        let width = cmd.chars().count() + 4 + 2 + 2;
        assert!(rendered.lines().all(|l| l.chars().count() == width));
    }

    #[test]
    fn log_line_plain_has_no_ansi_and_expected_tag() {
        let line = LogLine::ok(
            "deleted",
            "Configuration & cache removed  (~/.config/svgfetch)",
        );
        let rendered = line.render(false);
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.starts_with("✓ deleted "), "got {rendered:?}");
        assert!(rendered.contains("Configuration & cache"));
    }

    #[test]
    fn level_symbols_render_plain() {
        assert!(LogLine::skipped("x").render(false).starts_with("– skipped"));
        assert!(LogLine::notice("x").render(false).starts_with("! notice"));
        assert!(LogLine::failed("x").render(false).starts_with("! failed"));
    }

    #[test]
    fn tally_counts_levels() {
        let lines = vec![
            LogLine::ok("deleted", "a"),
            LogLine::skipped("b"),
            LogLine::notice("c"),
            LogLine::failed("d"),
        ];
        let mut tally = Tally::default();
        tally.add(&lines);
        assert_eq!(tally.done, 1);
        assert_eq!(tally.skipped, 1);
        assert_eq!(tally.warnings, 2);
        assert!(!tally.summary(false).contains('\u{1b}'));
    }

    #[test]
    fn thinking_frame_plain_is_ansi_free() {
        let frame = thinking_frame(3, "Working…", Duration::from_millis(1200), false);
        assert!(!frame.contains('\u{1b}'));
        assert!(frame.contains("Working…"));
        assert!(frame.contains("1.2s"));
    }

    #[test]
    fn thinking_frame_color_has_escapes() {
        let frame = thinking_frame(3, "Working…", Duration::from_millis(100), true);
        assert!(frame.contains('\u{1b}'));
    }

    #[test]
    fn download_frame_plain_is_ansi_free() {
        let frame = download_frame("svgfetch v1 → v2", 620, 1000, 3_100_000.0, 2.0, false);
        assert!(!frame.contains('\u{1b}'));
        assert!(frame.contains("62%"));
        assert!(frame.contains("eta"));
    }
}
