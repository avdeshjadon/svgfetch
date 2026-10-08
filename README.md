# svgfetch

Terminal-first client for discovering, inspecting, and downloading SVG assets from Wikimedia Commons. Includes both an interactive vector terminal interface and a scriptable CLI.

## Installation

### Using npm / npx

Run directly without installing:

```sh
npx svgfetch
```

Or install globally:

```sh
npm install -g svgfetch
```

### Pre-built binary (curl)

Install the pre-compiled binary for your operating system:

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.sh | bash
```

```powershell
# Windows PowerShell
irm https://raw.githubusercontent.com/avdeshjadon/svgfetch/main/install.ps1 | iex
```

The installer detects your OS and architecture, verifies the release checksum, and places the binary into `~/.local/bin`.

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

---

## Quick Start

### 1. Download Brand Logos Directly

Fetch verified vector assets directly by brand name:

```sh
svgfetch flipkart
svgfetch amazon
svgfetch google
svgfetch react
```

### 2. Smart Project Auto-Detection

When executed inside any frontend or web project (Next.js, React, Vite, Vue, Nuxt, Svelte, Astro, Angular), `svgfetch` automatically detects your project structure and places assets into your project's image directory:

- If `src/images/` or `src/assets/` exists: saved to `src/images/`
- If `public/images/` or `public/` exists: saved to `public/images/`
- Outside projects: saved to `~/Downloads/svgfetch/` or `./svgs/`

Manual directory specification via `-o` or `--output` is optional.

### 3. Interactive TUI Mode

Launch the interactive terminal interface:

```sh
svgfetch
```

- Type a search term and press Enter to search.
- Use arrow keys to navigate results.
- Press V to open vector preview in terminal.
- Press Space to select items for batch downloading.
- Press Enter to download the selected asset.
- Press Esc or q to exit.

---

## Command Reference

| Command | Description |
| --- | --- |
| `svgfetch <brand>` | Directly download a brand asset into the current project or folder |
| `svgfetch` | Launch full interactive terminal interface |
| `svgfetch info <asset>` | Inspect asset metadata (license, creator, dimensions, URL) without downloading |
| `svgfetch search <query>` | Search Commons without interactive UI |
| `svgfetch download <title>` | Download a specific file by name or title |
| `svgfetch category <name>` | List and download SVGs from a Commons category |
| `svgfetch batch <query>` | Search, download, and pack into a ZIP archive with attribution |
| `svgfetch cache <status\|clear>` | Inspect or prune the response cache |
| `svgfetch config <--init>` | View or generate default configuration |
| `svgfetch doctor` | Run diagnostics on environment, network, and configuration |
| `svgfetch update` | Update binary in place to the latest release |
| `svgfetch uninstall` | Cleanly remove configuration, cache, downloads, and binary |

### Global Flags

- `--dry-run`: Preview resolution and destination paths without writing any files or downloading assets.
- `--refresh`: Bypass cache and re-fetch fresh metadata from Wikimedia Commons.
- `--no-cache`: Completely disable cache lookups and storage for the command.
- `--project`: Explicitly enable project-aware placement into frontend asset folders.
- `--no-project`: Disable automatic project detection and use the default download directory.
- `-v, --verbose` / `--debug`: Display verbose/diagnostic runtime output.

### CLI Examples

Dry-run resolution without downloading:

```sh
svgfetch amazon --dry-run
```

Inspect asset metadata without downloading:

```sh
svgfetch info "File:GitHub_Logo.svg" --format json
```

Search and export results as JSON:

```sh
svgfetch search "logos" --limit 50 --format json
```

Download a specific file into a custom directory:

```sh
svgfetch download "File:GitHub_Logo.svg" -o ./assets
```

Batch search and compress into a ZIP archive with attribution:

```sh
svgfetch batch "minimalism" --zip assets.zip --metadata ./attribution -y
```

Clean uninstall:

```sh
svgfetch uninstall
# or alias
svgfetch dlt
```

---

## Configuration

Configuration file locations:
- macOS: `~/Library/Application Support/svgfetch/config.toml`
- Linux: `~/.config/svgfetch/config.toml`
- Windows: `%APPDATA%\svgfetch\config.toml`

Generate default configuration:

```sh
svgfetch config --init
```

Default settings:

```toml
download_directory = "~/Downloads/svgfetch"
max_concurrency = 4
cache_enabled = true
cache_ttl_hours = 24
cache_max_mb = 100
theme = "default"
animations = true
min_request_interval_ms = 250
max_response_mb = 16
max_download_mb = 100
project_detection = true
contact = "user@example.com"
```

---

## Attribution, Provenance, and Licensing

- Files downloaded from Wikimedia Commons retain their respective creator licenses (Creative Commons, Public Domain, etc.). SVGFetch dual-license (MIT / Apache-2.0) applies to the software itself, not to downloaded third-party assets.
- `svgfetch` writes structured, verifiable provenance and attribution metadata alongside downloaded assets:
  - `manifest.json`: Tool version, query, download timestamp, and file count.
  - `attribution.json`: Author, creator, license, license URL, and download timestamp for each asset.
  - `licenses.json`: Full license terms and URLs mapped to exact saved filenames.
  - `sources.json`: Original Wikimedia Commons page and direct asset URLs.
- Repeated downloads into the same folder merge metadata deterministically without destroying previous provenance entries.
- Requests respect Wikimedia API guidelines, using descriptive User-Agents, rate limiting, and exponential backoff on HTTP 429/5xx responses.

---

## Contributing

Contributions are welcome. Please ensure formatting, linting, and tests pass before opening a pull request:

```sh
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) for guidelines.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).