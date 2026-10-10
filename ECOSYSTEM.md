# Tri-Repository Ecosystem Architecture & Synchronization Guide

This document governs the multi-repository architecture connecting **`svgfetch`**, **`svgfetch-icons`**, and **`svgfetch-frontend`**.

> **CRITICAL RULE FOR ALL AI AGENTS AND DEVELOPERS**:  
> Whenever modifying ANY of the three repositories, you **MUST** ensure cross-repository compatibility and maintain strict synchronization contracts. A change in one repository often requires corresponding updates in the other two.

---

## 1. System Overview

```
                      +------------------------------------------+
                      |         svgfetch CLI / Rust Core         |
                      |  (Resolution Engine, Downloader, TUI)    |
                      +-------------------+----------------------+
                                          |
                        Priority 1 Query  |  Fallback Auto-Curate
                                          v
+-------------------------------+   Sync / PR   +-------------------------------+
|       svgfetch-icons          | <===========> |       Wikimedia Commons       |
|  - 24+ Categorized Folders    |               |  - Global open asset source   |
|  - manifest.json Index        |               |                               |
|  - CONTRIBUTIONS.md Log       |               +-------------------------------+
|  - jsDelivr / GitHub Raw CDN  |
+---------------+---------------+
                |
                | CDN Delivery
                v
+-------------------------------+
|      svgfetch-frontend        |
|  - React + Vite Web App       |
|  - Categorized Logo Library   |
|  - Marquee Showcase & Search  |
+-------------------------------+
```

### The Three Repositories

| Repository | Path | Role & Technology |
| :--- | :--- | :--- |
| **`svg-fetch`** | `/Users/avdeshjadon/svg-fetch` | **Core Engine & CLI** (Rust). Search, exact entity resolution, Wikimedia fallback, automatic curation, disk caching, and terminal UI. |
| **`svgfetch-icons`** | `/Users/avdeshjadon/svgfetch-icons` | **Asset Registry & CDN Source** (Zero-dependency Git repo). Stores 1,960+ categorized SVGs, `manifest.json`, and `CONTRIBUTIONS.md`. |
| **`svgfetch-frontend`** | `/Users/avdeshjadon/svgfetch-frontend` | **Web Showcase & Discovery Portal** (React 18 + Vite). Public web app allowing users to visually search, preview, and download icons via CDN. |

---

## 2. Synchronization Contracts

### Contract A: Asset Storage & Categorization (`svgfetch-icons`)
- **Structure**: All SVG assets reside under `logos/<category>/<filename>.svg`.
- **Naming Conventions**:
  - Full wordmark/logo: `<brand-slug>.svg` (e.g., `logos/e-commerce/myntra.svg`)
  - Standalone icon mark: `<brand-slug>-icon.svg` (e.g., `logos/microsoft/github-icon.svg`)
- **Manifest Index (`manifest.json`)**:
  - Automatically produced by `node scripts/generate-manifest.js`.
  - Maps each brand to `{ name, shortname, category, url, files: [ ... ] }`.
  - **Contract**: `files` array MUST contain relative paths including the category folder (e.g., `"e-commerce/myntra.svg"`).
- **Curation Log (`CONTRIBUTIONS.md`)**:
  - Every addition MUST be logged with UTC timestamp, brand name, category, relative file path, variant, and source provider.

### Contract B: CLI Resolution & Curation Pipeline (`svg-fetch`)
- **Resolution Order**:
  1. **Priority 1 (Curated CDN)**: Checks in-memory catalog -> local disk cache (`~/Library/Application Support/svgfetch/cache/curated_manifest.json`) -> bundled baseline (`data/curated_manifest.json`).
  2. **Priority 1 Live Sync**: If not found in cache, fetches real-time `manifest.json` from `raw.githubusercontent.com` (bypassing CDN propagation lag).
  3. **Priority 2 (Wikimedia Fallback)**: If brand is not curated, resolves via Wikimedia Commons API.
- **Auto-Contribution Pipeline (`src/curated/contribute.rs`)**:
  - When an asset is successfully downloaded from Wikimedia Commons:
    1. Classifies brand into proper category (e.g. `e-commerce`, `devops`, `fintech`, `social`, etc.).
    2. Saves SVG into `svgfetch-icons/logos/<category>/<brand>.svg`.
    3. Appends entry to `svgfetch-icons/CONTRIBUTIONS.md` with timestamp.
    4. Runs `node scripts/generate-manifest.js` to regenerate `manifest.json`.
    5. Syncs `svg-fetch/data/curated_manifest.json` and persistent disk cache.
    6. Commits and pushes `svgfetch-icons` to `origin main`.
- **Output Aesthetics**: Terminal output MUST remain clean and **strictly free of emojis or icons**. Use square bracket tags: `[curated]`, `[download]`, `[saved]`, `[sync]`, `[info]`.

### Contract C: Frontend Delivery & UI (`svgfetch-frontend`)
- **CDN Base URL**: `https://cdn.jsdelivr.net/gh/avdeshjadon/svgfetch-icons@main/logos`.
- **Nested Path Resolution**: In `LogoLibrary.jsx` and `LogoMarquee.jsx`, URLs are constructed using:
  ```javascript
  const fileUrl = (file) => `${CDN_BASE_URL}/${file.split('/').map(encodeURIComponent).join('/')}`;
  ```
- **Live Search Filtering**: Search query filters by `brand.name`, `brand.category`, and filename.
- **Clean Downloads**: When user clicks download in browser, file is downloaded with leaf name (`zomato.svg`), not nested category name.

---

## 3. Mandatory Rules for Developers & AI Agents

When working on any repository in this ecosystem, you MUST follow these steps:

### When Adding or Modifying a Brand:
1. **Never create flat files in `logos/`**: Always place SVGs inside their respective category directory: `logos/<category>/<brand>.svg`.
2. **Always regenerate manifest**: Run `npm run manifest` (or `node scripts/generate-manifest.js`) in `svgfetch-icons`.
3. **Always update `CONTRIBUTIONS.md`**: Record the addition with UTC timestamp.
4. **Synchronize `svg-fetch` manifest**: Copy `svgfetch-icons/manifest.json` into `svg-fetch/data/curated_manifest.json`.
5. **Run test suite**: In `svg-fetch`, execute `cargo test` and ensure all 124+ tests pass.
6. **Verify frontend build**: In `svgfetch-frontend`, execute `npm run build` to ensure no broken asset links or imports.

---

## 4. CI/CD & Deployment Workflow

```
[svgfetch-icons push to main]
            |
            v
  GitHub Action (.github/workflows/manifest.yml)
            |
            +--> Runs generate-manifest.js
            +--> Commits updated manifest.json (if needed)
            +--> Purges jsDelivr CDN cache (purge.jsdelivr.net)
            |
            v
  [svgfetch CLI Live Sync]
  - Fetches freshest manifest from raw.githubusercontent.com
  - Downloads SVGs from jsDelivr (or falls back to GitHub raw)
            |
            v
  [svgfetch-frontend Deployment]
  - Automatically reflects new manifest and CDN assets
```
