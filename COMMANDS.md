# svgfetch Command Reference (`COMMANDS.md`)

> **NOTICE FOR CONTRIBUTORS & AI AGENTS**:
> `COMMANDS.md` is the comprehensive, canonical reference for all CLI commands, subcommands, arguments, flags, keyboard shortcuts, and environment variables in `svgfetch`.
> **MANDATORY RULE**: Whenever a command, subcommand, option, or behavior is added, modified, or deprecated, this file **MUST** be updated synchronously.

---

## Table of Contents

- [Quick Command Cheatsheet](#quick-command-cheatsheet)
- [Primary Workflow: Direct Brand Resolution](#primary-workflow-direct-brand-resolution)
  - [Basic Syntax](#basic-syntax)
  - [Interactive Variant Selection & In-Terminal Preview](#interactive-variant-selection--in-terminal-preview)
  - [Natural Language Variant Syntax](#natural-language-variant-syntax)
  - [Explicit `--variant` Flags](#explicit---variant-flags)
  - [Project-Aware Placement Matrix](#project-aware-placement-matrix)
- [Global Flags & Options](#global-flags--options)
- [Subcommands In-Depth](#subcommands-in-depth)
  - [`svgfetch` (Interactive TUI)](#svgfetch-interactive-tui)
  - [`svgfetch search`](#svgfetch-search)
  - [`svgfetch download`](#svgfetch-download)
  - [`svgfetch category`](#svgfetch-category)
  - [`svgfetch batch`](#svgfetch-batch)
  - [`svgfetch info`](#svgfetch-info)
  - [`svgfetch suggest` / `request`](#svgfetch-suggest--request)
  - [`svgfetch cache`](#svgfetch-cache)
  - [`svgfetch config`](#svgfetch-config)
  - [`svgfetch doctor`](#svgfetch-doctor)
  - [`svgfetch update`](#svgfetch-update)
  - [`svgfetch uninstall` / `dlt`](#svgfetch-uninstall--dlt)
  - [`svgfetch version`](#svgfetch-version)
- [Interactive Keyboard Shortcuts](#interactive-keyboard-shortcuts)
- [Environment Variables](#environment-variables)
- [Exit Codes](#exit-codes)
- [Scripting & CI/CD Recipes](#scripting--cicd-recipes)

---

## Quick Command Cheatsheet

| Command | Purpose | Example |
| :--- | :--- | :--- |
| `svgfetch` | Launch full-screen interactive TUI with search & preview | `svgfetch` |
| `svgfetch <brand>` | Fast direct brand download with variant picker | `svgfetch amazon` |
| `svgfetch <brand> icon` | Download icon mark using natural language | `svgfetch instagram icon` |
| `svgfetch <brand> --variant <v>` | Download specific variant (`icon`, `wordmark`, `full`, `mascot`) | `svgfetch docker --variant full` |
| `svgfetch <brand> --dry-run` | Inspect destination and asset without writing | `svgfetch stripe --dry-run` |
| `svgfetch search <query>` | Search Wikimedia Commons with tabular/JSON output | `svgfetch search "react" -n 10` |
| `svgfetch download <file>` | Download specific file by filename or MediaWiki title | `svgfetch download "File:Rust_logo.svg"` |
| `svgfetch category <name>` | Browse and download SVGs from Commons category | `svgfetch category "Vector logos"` |
| `svgfetch batch <query>` | Search, download, and pack into ZIP with licensing | `svgfetch batch "minimalism" --zip out.zip` |
| `svgfetch info <asset>` | Inspect license, author, and dimensions | `svgfetch info github` |
| `svgfetch suggest <brand>` | Request missing brand for auto-curation in `svgfetch-icons` | `svgfetch suggest "Notion"` |
| `svgfetch cache <status\|clear>` | Check disk usage or clear HTTP response cache | `svgfetch cache clear` |
| `svgfetch config [--init]` | View or initialize default configuration file | `svgfetch config --init` |
| `svgfetch doctor` | Diagnose network, terminal TrueColor, and dependencies | `svgfetch doctor` |
| `svgfetch update` | In-place self-update to latest GitHub release | `svgfetch update` |
| `svgfetch uninstall` / `dlt` | Completely wipe binaries, settings, and cache | `svgfetch uninstall -y` |

---

## Primary Workflow: Direct Brand Resolution

### Basic Syntax

```sh
svgfetch <brand_name> [OPTIONS]
```

- Multi-word queries do not require quotes:
  ```sh
  svgfetch amazon prime
  svgfetch kali linux
  ```
- Priority resolution order:
  1. **Bundled Baseline Catalog**: 1,480+ curated brands loaded in memory (0ms latency).
  2. **Live CDN Sync**: Fetches newest additions from `svgfetch-icons` if brand is new.
  3. **Wikimedia Commons Semantic Search**: 8-tier ranking engine for non-curated brands.

---

### Interactive Variant Selection & In-Terminal Preview

When a curated brand has multiple assets (e.g., `Instagram` has both a Full Logo and an Icon Mark) and you run without `--variant` in a TTY:

```text
[curated] Found "Instagram" with 2 variants.
Use [↑/↓] to navigate, [v] to preview, [Enter] to select & download, [Esc/q] to cancel:

  > 1. Full Logo   (instagram.svg)       [Press 'v' to preview]
    2. Icon Mark   (instagram-icon.svg)  [Press 'v' to preview]
```

#### Controls:
- **`↑` / `↓`** (or `k` / `j`, or numbers `1` - `9`): Move active cursor.
- **`v`**: Renders a **24-bit TrueColor ANSI vector artwork preview** directly inside your terminal using Unicode half-blocks (`▀`, `▄`, `█`), and opens an infinite-zoom retina vector window.
- **`Enter`**: Confirms variant and proceeds to download.
- **`Esc` / `q`** (or `Ctrl+C`): Cancels operation without downloading.

---

### Natural Language Variant Syntax

You can append variant keywords directly into the command:

```sh
# Icon / Symbol
svgfetch instagram icon
svgfetch github symbol

# Wordmark / Logotype
svgfetch instagram wordmark
svgfetch google text

# Full Logo / Lockup
svgfetch docker full
svgfetch aws lockup

# Mascot / Character
svgfetch linux mascot
svgfetch linux character
```

---

### Explicit `--variant` Flags

Use the `--variant` flag to bypass interactive prompts in automated scripts or CLI pipelines:

| Flag Value | Aliases | Description |
| :--- | :--- | :--- |
| `--variant default` | `canonical`, `best` | Default canonical brand asset |
| `--variant icon` | `symbol`, `mark`, `glyph` | Icon, emblem, or mark only |
| `--variant wordmark` | `logotype`, `text` | Text-based brand name |
| `--variant full` | `horizontal`, `lockup` | Complete logo + wordmark combined |
| `--variant mascot` | `character` | Entity mascot or illustrated character |

**Example**:
```sh
svgfetch docker --variant full
svgfetch python --variant icon
```

---

### Project-Aware Placement Matrix

When invoked inside a project directory, SVGFetch inspects the directory hierarchy and places downloaded SVGs into the appropriate assets folder automatically:

| Detected Project | Marker Files | Default Target Placement |
| :--- | :--- | :--- |
| **Next.js** | `next.config.*` | `./public/` |
| **Vite** | `vite.config.*` | `./public/` or `./src/assets/` |
| **React** | `src/` + `package.json` | `./src/assets/` or `./public/` |
| **Vue / Nuxt** | `nuxt.config.*`, `vue.config.*` | `./public/` or `./assets/` |
| **Svelte / SvelteKit** | `svelte.config.*` | `./static/` |
| **Astro** | `astro.config.*` | `./public/` or `./src/assets/` |
| **Angular** | `angular.json` | `./src/assets/` |
| **Web / Node** | `package.json` | `./public/` or `./assets/` |
| **Non-Project / Root** | None | `~/Downloads/svgfetch/` |

#### Overriding Placement:
- Force project placement: `svgfetch react --project`
- Disable project placement: `svgfetch react --no-project` (saves to `~/Downloads/svgfetch/`)
- Specify exact destination: `svgfetch react -o ./custom/path/logo.svg`

---

## Global Flags & Options

These flags apply across commands and direct downloads:

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--variant <TYPE>` | - | Choose asset variant: `default`, `icon`, `wordmark`, `full`, `mascot` |
| `--output <PATH>` | `-o` | Destination directory or filename |
| `--overwrite` | - | Overwrite existing files instead of skipping |
| `--dry-run` | - | Simulate operation without downloading or writing files |
| `--project` | - | Force project-context placement (conflicts with `--no-project`) |
| `--no-project` | - | Bypass project detection and use default download directory |
| `--refresh` | - | Force-bypass cache and fetch live data |
| `--no-cache` | - | Disable cache reading and writing |
| `--verbose` | `-v` | Enable detailed info logs |
| `--debug` | - | Enable technical debug logs and backtraces |
| `--help` | `-h` | Display help and usage information |
| `--version` | `-V` | Display version information |

---

## Subcommands In-Depth

### `svgfetch` (Interactive TUI)

Launches the full-screen interactive Terminal UI (Ratatui + Crossterm). Requires a real TTY.

```sh
svgfetch
```

- Type queries in real time.
- Instant 24-bit TrueColor image cards on high-contrast surfaces.
- Multi-select assets with `Space` and batch download or pack into ZIP.

---

### `svgfetch search`

Non-interactive search on Wikimedia Commons.

```sh
svgfetch search <query> [OPTIONS]
```

#### Options:
- `-n, --limit <N>`: Maximum results (default: `25`).
- `-f, --format <table|json|jsonl>`: Output format (default: `table`).
- `--download <DIR>`: Download matching assets into directory.
- `--zip <FILE>`: Pack results into a `.zip` archive.
- `--metadata <DIR>`: Save license & attribution files (`attribution.json`, `attribution.md`).
- `--license <FILTER>`: Case-insensitive filter (e.g. `cc-by`, `public domain`).
- `--page <N>`: Page number (default: `1`).
- `-y, --yes`: Skip confirmation prompts for bulk downloads.
- `--overwrite`: Overwrite existing files.
- `--concurrency <N>`: Parallel download threads (1 - 32).

**Examples**:
```sh
# Search and display JSON
svgfetch search "docker" -n 5 -f json

# Search, download first 20, and write attribution metadata
svgfetch search "social icons" -n 20 --download ./icons --metadata ./icons/meta -y
```

---

### `svgfetch download`

Download a specific SVG by exact file name or MediaWiki title.

```sh
svgfetch download <file_or_title> [OPTIONS]
```

#### Options:
- `-o, --output <DIR>`: Output directory (defaults to settings download dir).
- `-f, --format <table|json|jsonl>`: Result output format.
- `--overwrite`: Overwrite existing file.

**Examples**:
```sh
svgfetch download "File:Rust_logo.svg"
svgfetch download "File:GitHub_Logo.svg" -o ./public
```

---

### `svgfetch category`

List and download SVGs belonging to a Wikimedia Commons Category.

```sh
svgfetch category <category_name> [OPTIONS]
```

- Accepts names with or without `Category:` prefix (e.g. `Vector logos` or `Category:Vector logos`).

#### Options:
- `-n, --limit <N>`: Max items (default: `25`).
- `-f, --format <table|json|jsonl>`: Output format.
- `--download <DIR>`: Download assets into directory.
- `--zip <FILE>`: Pack results into ZIP.
- `-y, --yes`: Skip confirmation.
- `--overwrite`: Overwrite existing files.

**Example**:
```sh
svgfetch category "Google logos" -n 15 --download ./google_logos -y
```

---

### `svgfetch batch`

Search and download bulk assets in one automated command.

```sh
svgfetch batch <query> [OPTIONS]
```

#### Options:
- `-o, --download <DIR>`: Download directory.
- `--zip <FILE>`: Compress directly into a ZIP archive.
- `--metadata <DIR>`: Save attribution documentation.
- `-n, --limit <N>`: Maximum assets (default: `50`).
- `-y, --yes`: Skip bulk confirmation.
- `--overwrite`: Overwrite existing files.

**Example**:
```sh
svgfetch batch "payment methods" --zip payments.zip --metadata ./meta -y
```

---

### `svgfetch info`

Inspect asset metadata without downloading. Displays license, author, source page URL, dimensions, and confidence scoring.

```sh
svgfetch info <asset_or_query> [OPTIONS]
```

#### Options:
- `--variant <TYPE>`: Specific variant (`icon`, `wordmark`, `full`, `mascot`).
- `-f, --format <table|json|jsonl>`: Output format.

**Examples**:
```sh
svgfetch info instagram
svgfetch info instagram --variant icon
svgfetch info "File:GitHub_Logo.svg" -f json
```

---

### `svgfetch suggest` / `request`

Suggest a missing brand for automatic curation into the official `svgfetch-icons` ecosystem.

```sh
svgfetch suggest <brand> [OPTIONS]
# Alias:
svgfetch request <brand> [OPTIONS]
```

- Fires a GitHub Actions `repository_dispatch` event on `avdeshjadon/svgfetch-icons`.
- CI autonomously fetches from Wikimedia, validates security, updates `CONTRIBUTIONS.md`, regenerates manifest, commits, pushes, and purges CDN.
- Requires `SVGFETCH_GITHUB_TOKEN` environment variable (repo scope on `svgfetch-icons`).

#### Options:
- `-m, --message <NOTE>`: Optional note or context stored in dispatch payload.
- `-c, --category <CAT>`: Override auto-detected category (e.g. `devops`, `ai`, `e-commerce`).
- `--dry-run`: Print payload without dispatching.

**Examples**:
```sh
svgfetch suggest "Linear" -m "Design system icons"
svgfetch suggest "SuperApp" --category devops --dry-run
```

---

### `svgfetch cache`

Inspect and manage the local HTTP response cache.

```sh
svgfetch cache <status|clear>
```

#### Commands:
- `svgfetch cache status`: Shows cache location, number of cached files, and disk size.
- `svgfetch cache clear`: Removes all cached responses.

---

### `svgfetch config`

Inspect or initialize configuration (`config.toml`).

```sh
svgfetch config [OPTIONS]
```

#### Options:
- `--init`: Generates a clean default `config.toml` if not already present.

---

### `svgfetch doctor`

Performs diagnostic health checks on your environment:
- Network connectivity to Wikimedia API and jsDelivr CDN.
- Terminal TrueColor 24-bit capability.
- Config directory permissions and disk space.
- Image rasterizer / `resvg` availability.

```sh
svgfetch doctor
svgfetch doctor --json
```

---

### `svgfetch update`

Checks GitHub releases for `svgfetch`, downloads the platform-specific precompiled binary, verifies SHA-256 checksum, and replaces the running binary in-place.

```sh
svgfetch update
```

---

### `svgfetch uninstall` / `dlt`

Completely removes `svgfetch` from your system leaving no residual files:
- Removes configuration file (`config.toml`), settings, and history.
- Removes response cache directory.
- Removes default downloads folder (`~/Downloads/svgfetch`).
- Removes binary and global npm wrappers.

```sh
svgfetch uninstall
# Alias:
svgfetch dlt -y
```

---

### `svgfetch version`

Displays version, git commit hash, and target architecture.

```sh
svgfetch version
svgfetch -V
svgfetch --version
```

---

## Interactive Keyboard Shortcuts

### 1. Variant Picker (Direct Command Mode)
When running `svgfetch <brand>` for multi-variant brands:

| Key | Action |
| :--- | :--- |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `1` - `9` | Jump directly to item number |
| `v` / `V` | **Preview**: Renders ANSI half-block artwork in terminal & launches retina window |
| `Enter` | Select active variant and download |
| `Esc` / `q` / `Ctrl+C` | Cancel without downloading |

### 2. Full-Screen Interactive TUI (`svgfetch`)

| Key | Action |
| :--- | :--- |
| `↑` / `↓` / `j` / `k` | Navigate search results list |
| `PgUp` / `PgDn` | Scroll results page-by-page |
| `Home` / `End` | Jump to first / last item |
| `Enter` | Open asset details screen |
| `Space` | Toggle multi-select checkbox for batch download |
| `a` | Select / deselect all items on page |
| `d` | Download selected item (or all checked items) |
| `z` | Pack selected items into a ZIP archive |
| `v` | Open infinite-zoom retina vector viewer in external browser |
| `t` | Cycle visual card background theme (Light / Checkerboard / Dark) |
| `/` | Focus search query input bar |
| `r` | Refresh query bypassing cache |
| `?` | Toggle help overlay cheatsheet |
| `Esc` / `q` | Go back one screen / Exit |

### 3. Retina Vector Window (Browser Viewer)

| Key | Action |
| :--- | :--- |
| `Scroll Wheel` / `Pinch` | Zoom in / out (up to 3,000%) centered on mouse pointer |
| `Click & Drag` | Pan canvas |
| `+` / `-` | Keyboard zoom in / out |
| `0` / `r` | Reset zoom and pan to 100% |
| `t` | Cycle canvas theme (White Grid / Checkerboard / Dark) |
| `Esc` / `q` | Close window |

---

## Environment Variables

| Variable | Description |
| :--- | :--- |
| `SVGFETCH_GITHUB_TOKEN` | GitHub Personal Access Token (PAT) used by `svgfetch suggest` to trigger CI curation. |
| `SVGFETCH_CONFIG_DIR` | Custom directory for `config.toml`. Defaults to `~/.config/svgfetch`. |
| `SVGFETCH_CACHE_DIR` | Custom directory for cache database. Defaults to `~/.cache/svgfetch`. |
| `SVGFETCH_DOWNLOAD_DIR`| Default destination for non-project downloads. Defaults to `~/Downloads/svgfetch`. |
| `SVGFETCH_USER_AGENT` | Custom HTTP `User-Agent` string for Wikimedia Commons compliance. |
| `SVGFETCH_UPDATE_REPO` | Override GitHub repo for self-update (defaults to `avdeshjadon/svgfetch`). |

---

## Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | Success / Operation completed normally |
| `1` | Asset not found / General error / User cancelled prompt |
| `2` | Network or connection failure |
| `3` | File system or I/O permission error |

---

## Scripting & CI/CD Recipes

### 1. Silent Download in Docker / Shell Scripts
```sh
# Bypass prompts, force overwrite, and output to specific directory
svgfetch stripe --variant icon -o ./assets/icons --overwrite --no-project
```

### 2. Extract CDN URL Without Downloading
```sh
svgfetch info amazon --variant icon -f json | jq -r '.url'
```

### 3. Automated Project Asset Sync (Makefile / npm script)
```json
{
  "scripts": {
    "sync:icons": "svgfetch react -o ./src/assets/react.svg && svgfetch tailwind -o ./src/assets/tailwind.svg"
  }
}
```
