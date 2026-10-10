# AI Agent Operating Guidelines: svgfetch

> **CRITICAL INSTRUCTION**: This repository (`svg-fetch`) is part of a 3-repo ecosystem:
> 1. `svg-fetch` (This repo: Rust CLI core)
> 2. `svgfetch-icons` (Sibling directory `../svgfetch-icons` / GitHub: `avdeshjadon/svgfetch-icons`)
> 3. `svgfetch-frontend` (Sibling directory `../svgfetch-frontend` / GitHub: `avdeshjadon/svgfetch-frontend`)
>
> You MUST read [`ECOSYSTEM.md`](ECOSYSTEM.md) before making architectural changes.

## Mandatory Agent Rules:

1. **Maintain Ecosystem Synchronization**:
   - Whenever `data/curated_manifest.json` changes, `svgfetch-icons/manifest.json` must be in sync.
   - Any newly curated brand must be saved under its category path `svgfetch-icons/logos/<category>/<brand>.svg`.
   - Manifest regeneration (`node scripts/generate-manifest.js`) and log update (`CONTRIBUTIONS.md`) are mandatory.

2. **Terminal Output Constraints**:
   - Keep all CLI terminal messages **strictly clean and free of emojis/icons**.
   - Use standardized prefix tags: `[curated]`, `[download]`, `[saved]`, `[sync]`, `[info]`, `[warn]`, `[error]`.

3. **Validation & Quality Gates**:
   - Always run `cargo test` before submitting changes (all 124+ unit, property, and integration tests must pass).
   - Ensure `cargo fmt --check` and `cargo check` report 0 errors.

4. **Preserve Security & Path Sanitization**:
   - All filenames must continue to pass through `core::security::safe_join` and `sanitize_filename`.
   - Never write outside the user-specified directory.
