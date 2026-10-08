# svgfetch Official Website

Official website and interactive web playground for **svgfetch** — the pure-Rust CLI tool to discover, inspect, and download SVG assets from Wikimedia Commons.

Built with **React**, **Vite**, and **Vanilla CSS Design Tokens** for maximum speed, minimalist developer aesthetics (inspired by npm, Vercel, and Bun), and zero bloated CSS dependencies.

---

## ⚡ Features

- **Interactive Playground**: Search top brands, toggle exact variants (`icon`, `wordmark`, `full`, `mascot`), copy CLI commands, or copy raw SVG vectors in real time.
- **npm-Style Quick Install**: Instant copy-to-clipboard for `npx`, `npm i -g`, `cargo`, `brew`, `curl`, and `PowerShell`.
- **Terminal Simulator**: Visual recreation of the `svgfetch` interactive TUI, halfblock vector previews, and diagnostic output.
- **Entity Matching Guide**: Real-world examples showing how svgfetch prevents fuzzy mismatches (e.g. `Facebook != Meta`, `Linux != Tux`).
- **Complete CLI Reference**: Searchable commands table with descriptions, aliases, and flags matrix.
- **Integrity & Licensing**: Details on Path Traversal/Zip Slip defense, provenance metadata, and GNU AGPLv3 copyleft terms.

---

## 🛠 Local Development

```bash
# Navigate to the website directory
cd svgfetch

# Install dependencies
npm install

# Start development server
npm run dev
```

Server runs locally at [http://localhost:5173/](http://localhost:5173/).

---

## 📦 Production Build

```bash
npm run build
```

The production-ready static assets will be emitted in the `dist/` directory.

---

## 🚀 Deployment Options

### Vercel
```bash
npx vercel
```
* **Framework Preset**: Vite
* **Root Directory**: `svgfetch`
* **Build Command**: `npm run build`
* **Output Directory**: `dist`

### Netlify
```bash
npx netlify deploy --prod --dir=dist
```

### GitHub Pages
Deploy the `svgfetch/dist` directory using `gh-pages` or a GitHub Actions workflow.

---

## 📄 License

Licensed under the **GNU Affero General Public License v3.0** (AGPL-3.0).
