# Changelog

All notable changes to this project are documented in this file. This project
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.6] - 2026-10-09

### Added
- **Exact Asset Variants**: Added `--variant <VARIANT>` option supporting `default`, `icon`, `wordmark`, `full`, and `mascot` with friendly aliases (`symbol`, `mark`, `text`, `logotype`, `lockup`, `character`).
- **Natural Language Variant Parsing**: Automatically extract variant keywords from queries (`svgfetch instagram wordmark`, `svgfetch linux mascot`) with explicit CLI flag precedence.
- **Exact Entity Resolution**: Entity-first resolution prioritizing exact entity semantics (e.g. Facebook != Meta, Instagram != Meta, Kali Linux != Linux, Linux != Tux unless mascot requested).
- **Curated Multi-Variant Registry**: Added variant file mappings for major brands (Instagram, Docker, GitHub, Linux, Kali Linux, Facebook, Meta, Python, Ubuntu) in `brands.json`.
- **Smart Hints & Suggestions**: Contextual variant recommendations for ambiguous queries and safe unknown query handling preventing random SVG downloads.
- **Enhanced Info Transparency**: `svgfetch info <query>` reports query, resolved entity, entity type, variant, selected asset, confidence percentage, and reason.

## [0.2.5] - 2026-10-08

### Added
- **Unified Secure Download Pipeline**: Centralized streaming downloads with configurable file limits (`max_download_mb`), temporary `.part` file writes, atomic rename, and SHA-256 calculation.
- **Content Validation**: SVG XML validation checking root namespaces and structure while rejecting HTML error pages, PE/ELF executables, and corrupted payloads.
- **Dry-run Mode**: Global `--dry-run` flag to preview downloads, resolution, and targets without touching disk.
- **Info Command**: `svgfetch info <asset>` to inspect dimensions, licenses, authors, and source URLs without downloading.
- **Metadata Persistence**: Verifiable provenance records in `manifest.json`, `attribution.json`, `licenses.json`, and `sources.json`, preserving `saved_filename` vs `source_filename`, SHA-256, and multi-run merging.
- **Cache Management**: Added `--refresh` and `--no-cache` flags with limit-aware cache keys.
- **Project Overrides**: Added `--project` and `--no-project` flags for controlling automatic project detection.
- **npm Package Verification**: Added prebuilt binary SHA-256 checksum verification in `npm/bin/install.js`.
- **Single-Pass ZIP**: Direct local archiving from downloaded files when combining `--download` and `--zip`.

### Changed
- Clarified brand nomenclature to "Curated brand mapping".
- Added system platform and binary version inspection to `svgfetch doctor`.
- Removed `continue-on-error: true` from npm release publishing.

### Changed
- Better `search` relevance: a single bare token like `github` or `amazon`
  is now scoped with `intitle:` so results are far more on-point (previously a
  phrase-search matched the token anywhere on the page). Multi-word phrases
  and explicit operators are left untouched.
- Merged direct **Download** into the **search input**: type a `File:...`
  title or a name ending in `.svg` and press Enter to hop straight to
  download, otherwise it is a normal search. The search box hint and the
  search-hint line now reflect both behaviours.

### Added
- Smart routing in the TUI search input (`looks_like_file`), reusing the
  existing guided-download pipeline (`resolve_file`).

### Fixed
- The `d`/`D` home shortcut now correctly routes through search semantics
  instead of a leftover `DownloadInput` path mapping.

## [0.2.4] - 2026-10-08

### Added
- **Smart Project Detection**: Automatically detects React, Next.js, Vue, Nuxt, Svelte, Vite, and Node projects and downloads to `src/images/` or `public/images/`.
- **Complete Uninstall Command**: `svgfetch uninstall` (alias `dlt`) cleanly wipes configs, caches, download folder, and global npm packages.

## [0.2.3] - 2026-10-02

### Changed
- **Minimal search-first TUI with ASCII Logo Banner**: Launching `svgfetch` directly displays the SVGFETCH block-letter banner with tagline and the search input box immediately below it without any cluttered menus.
- **Detailed Asset View**: Pressing `Enter` on any search result opens a comprehensive details screen showing license, dimensions, file size, direct SVG download URL, and direct canonical Wikimedia Commons web link.
- **Confirmation Prompts for Downloads**:
  - **Complete ZIP Archive**: Added `Download all files as complete ZIP` action with `[Y/N]` confirmation dialog.
  - **Individual File Download**: In the details view, added `[Y/N]` prompt (`y` / `Enter` to download only that file, `n` / `Esc` to return to results).

### Added
- Keybindings in results: `z` triggers ZIP archive confirmation; `y`/`n` to confirm/cancel.
- Keybindings in details: `y`/`Enter` to download single file; `n`/`Esc` to return to results; `↑`/`↓` to navigate between previous/next files.

## [0.2.1] - 2026-09-21

### Added
- Home screen **Download a file** menu entry plus a `d` shortcut that opens a
  dedicated download prompt (accepts `File:Title` or `name.svg`).
- `svgfetch update` self-update subcommand with release fetching, checksum
  verification (SHA-256), atomic swap, and cleanup of old binaries.
- `download` route in the search box hinting you can fetch by exact name.

### Fixed
- TUI now keeps remote-ready assets per search and avoids dropping partially
  downloaded assets on screen changes.


### Added
- Interactive TUI (`svgfetch` with no arguments) with search, multi-select,
  live download/ZIP progress, and an attribution/details view.
- Provider abstraction (`AssetProvider`) designed so a second SVG
  source can be added without touching UI or download code.
- Non-interactive commands: `search`, `category`, `batch`, `download`,
  `config`, `cache status|clear`, `doctor`, `version`.
- Machine-readable output: `--format table|json|jsonl`.
- Bulk ZIP packing with an included `ATTRIBUTION.md` and per-file license
  metadata via `--metadata`.
- On-disk search-result cache with TTL and size budget; `cache status`/`clear`.
- Case-insensitive, collision-safe filename allocation (`file.svg`, `file-1.svg`).

### Security
- Central sanitizers in `src/security.rs`: filename sanitization, safe path
  joining, ANSI/OSC/control-character stripping, markup flattening, and strict
  HTTPS URL validation.
- Response-size ceilings on API bodies (16 MB default / 64 MB hard) and 4 GiB
  per-file download cap.
- 429/`Retry-After` and exponential backoff, plus a per-process request
  interval limit.
- Filenames and `contact` headers are scrubbed so untrusted input cannot
  escape their container (no `..`, no control chars, no header injection).
- Japanese/multibyte-safe truncation that never splits UTF-8 and preserves
  file extensions.

### Changed
- Search queries automatically append `filemime:image/svg+xml` and constrain
  to the `File` namespace (6) unless already constrained.
- `create_zip` accepts an optional `FnMut` event callback so callers can render
  progress (breaking API change from the `&mut dyn` signature).

### Fixed
- Specialized handling of `File:`/`image:` wiki links so captions (not raw
  filenames) survive `strip_markup`.
- Trailing/leading dot abuse in user-supplied names can no longer leak `..`
  components into output paths.
- Query strings with quotes cannot break out of the `incategory:"…"` clause.

### Removed
- Dead `theme` configuration fields (name/title colors/high-contrast) and the
  unused `dv` render helper; keymap fields are now fully wired to handlers.

## [0.1.0] - 2026-09

Initial public release.

[0.1.0]: https://github.com/avdeshjadon/get-svg/releases/tag/v0.1.0