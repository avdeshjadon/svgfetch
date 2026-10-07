# svgfetch

**svgfetch** is a terminal-first client for discovering, inspecting, and downloading SVG
assets from [Wikimedia Commons](https://commons.wikimedia.org). It ships both a
fast, keyboard-driven terminal interface and a scriptable CLI that speaks `table`, `JSON`,
and JSON-lines — with downloads, ZIP packing, per-file attribution metadata, caching,
and a rate limiter that stays polite toward Wikimedia's shared infrastructure.

> `svgfetch — Discover. Download. Ship SVGs.`

## Features

- **Interactive TUI** — launch straight into search: type a keyword like
  `Amazon` (no `.svg` extension needed) and get relevant SVG results, then
  either **Download Manually** (check-box selection screen) or **Download as
  ZIP** (one archive of every result). No menus, no home screen.
- **Scriptable CLI** — `search`, `category`, `batch`, `download`, `config`, `cache`, `doctor`.
  `--format table|json|jsonl` for pipelines.
- **Bulk downloads** — parallel (polite, capped) downloads with atomic writes and
  collision-safe naming (`file.svg`, `file-1.svg`, …), case-insensitive on disk.
- **ZIP packing** — one archive per query with an included `ATTRIBUTION.md`.
- **Attribution metadata** — a `metadata/` folder with per-file license and credit
  records, so exporting art stays legally clean.
- **On-disk cache** — search results cached with TTL + size budget; `svgfetch cache status|clear`.
- **Safety first** — path-traversal-proof filenames, HTTPS-only, response-size ceilings,
  429/retry backoff, and terminal escape stripping.

## Installation

### via npm / npx (Zero setup)

Run instantly without installing:

```sh
npx svgfetch
```

Or install globally via npm:

```sh
npm install -g svgfetch
```

### Pre-built binaries (recommended)

One-line installers fetch the [latest GitHub release](https://github.com/avdeshjadon/svgfetch/releases)
for your OS + CPU and verify its SHA-256 checksum before installing to
`~/.local/bin`:

```sh
# macOS / Linux / Windows Git Bash / MSYS
curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | sh
```

```powershell
# Windows PowerShell (native)
Set-ExecutionPolicy -Scope Process Bypass
irm https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.ps1 | iex
```

- The latest release is resolved automatically — no version to remember. To pin
  one anyway, set `SVGFETCH_VERSION=v0.2.3` (or `$env:SVGFETCH_VERSION`).
- The installer puts `svgfetch` (and aliases) into `~/.local/bin`. After it finishes
  (and you add the printed directory to your `PATH`) just type `svgfetch` to launch
  the interactive interface, or `svgfetch --help` for the full command list.
- Linux builds target the GNU C library (glibc ≥ 2.31, e.g. Ubuntu 20.04+,
  Debian 11+). macOS binaries are unsigned — install via the command line
  (curl) to avoid Gatekeeper prompts.
- Re-run with `--dir "$HOME/bin"` / `INSTALL_DIR` to install somewhere else.

### From source

```sh
cargo install svgfetch --locked
```

Or build from source:

```sh
git clone https://github.com/avdeshjadon/svgfetch.git
cd svgfetch
cargo build --release
```

## Quick start

Launch the interactive interface (requires a real TTY):

```sh
svgfetch
```

Non-interactive search:

```sh
svgfetch search "github logo"
svgfetch search "logos" --limit 50 --format json
svgfetch category "Logos" --limit 100 --download ./logos
```

Download one file and its metadata:

```sh
svgfetch download "File:GitHub_Logo.svg" -o ./assets
```

Search + download + zip + write attribution in one step:

```sh
svgfetch batch "minimalism" --zip repo-svg.zip --metadata ./attribution -y
```

Inspect and maintain the cache:

```sh
svgfetch cache status
svgfetch cache clear
```

Self-diagnose environment, network, and config:

```sh
svgfetch doctor
```

Update to the latest release:

```sh
svgfetch update
```

## Command reference

Run `svgfetch <command> --help` for the authoritative flag list. Overview:

| Command      | Purpose                                            |
| ------------ | -------------------------------------------------- |
| `search`     | Search Commons for SVGs; optional download/zip/metadata. |
| `category`   | List SVGs in a category (with or without the `Category:` prefix). |
| `batch`      | Search and download/zip/metadata everything in one step. |
| `download`   | Download a single file by name or `File:` title.   |
| `cache`      | Show or clear the local response cache.            |
| `config`     | Show the effective config; `--init` writes a default file. |
| `doctor`     | Environment/network/config diagnostics (`--json`). |
| `version`    | Print version info.                                |
| `update`     | Download + install the latest release in place.    |
| `dlt`        | Completely remove svgfetch (config, cache, downloads, binaries). |

Interactive keyboard map (built into the footer hints):

| Key | Action |
| --- | ------ |
| Search input | Type a keyword, `Enter` to search, `Esc` to clear |
| Results | `↑↓` navigate, `Space` toggle, `Enter` open action, `Esc` new search |
| `⬇ Download Manually` | Select screen: `Space` toggle, `Enter` download selected |
| `⬇ Download as ZIP` | Packs every result into one ZIP |
| `q` / `Ctrl+C` | Quit |

Uninstall completely — deletes config, cache, recent searches, the default
`~/Downloads/svgfetch` folder, and the binaries themselves:

```sh
svgfetch dlt          # asks for confirmation
svgfetch dlt --yes    # non-interactive
```

## Configuration

`svgfetch` reads TOML from your platform config dir:

- Linux: `~/.config/svgfetch/config.toml`
- macOS: `~/Library/Application Support/svgfetch/config.toml`
- Windows: `%APPDATA%\svgfetch\config.toml`

Generate a file with all defaults:

```sh
svgfetch config --init
```

`config --init` prints the current effective config; `svgfetch config` (no flag) is
read-only. Available keys:

```toml
download_directory = "~/Downloads/svgfetch"
max_concurrency = 4          # parallel downloads (1-32)
cache_enabled = true
cache_ttl_hours = 24
cache_max_mb = 100
theme = "default"
animations = true
min_request_interval_ms = 250  # minimum gap between API request starts
max_response_mb = 16           # hard ceiling per API response
contact = "your@email"         # appended to the User-Agent (recommended for heavy use)
```

The default User-Agent follows [Wikimedia policy](https://meta.wikimedia.org/wiki/User-Agent_policy):
`svgfetch/<version> (<repository>; <contact>)`. Setting `contact` is recommended if you
run heavy automation so Wikimedia can reach you.

## License, attribution, and Wikimedia etiquette

- Files on Wikimedia Commons carry their **own** licenses. svgfetch never invents one:
  per-file license/credit info is written next to downloaded files (`metadata/`),
  and unknown licenses are reported as `Unknown` rather than guessed.
- The client enforces a conservative request rate, responds to `429 Retry-After`, and
  limits concurrency so bulk jobs do not hammer the API.

## Development

```sh
cargo fmt --check          # formatting
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all           # unit + property + mock-server integration tests
cargo bench                # security-path benchmarks
cargo audit                # dependency vulnerability scan
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and the
[CHANGELOG.md](CHANGELOG.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). You may choose
either license.