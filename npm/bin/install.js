#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const os = require("os");
const { execSync } = require("child_process");
const crypto = require("crypto");

const pkg = require("../package.json");
const VERSION = `v${pkg.version}`;
const REPO = "avdeshjadon/svgfetch";

const tty = require("tty");

// --- presentation ---------------------------------------------------------
// Cosmetic only: every message goes to terminal TTY if available (bypassing npm pipe suppression),
// and falls back to stderr. All colour/animation is disabled when not a TTY,
// when NO_COLOR is set, or when TERM=dumb.
const TICK_MS = 70;
const MIN_STEP_MS = 450;

const ttyPath = process.platform === "win32" ? "\\\\.\\CONOUT$" : "/dev/tty";
let termFd = null;
try {
  termFd = fs.openSync(ttyPath, "w");
} catch (_) {}

const isTTY =
  termFd !== null
    ? tty.isatty(termFd)
    : (!!process.stderr.isTTY || !!process.stdout.isTTY);
const NO_COLOR = Object.prototype.hasOwnProperty.call(process.env, "NO_COLOR");
const TERM_DUMB = process.env.TERM === "dumb";
const useColor = isTTY && !NO_COLOR && !TERM_DUMB;
const useAnim = useColor;

function sgr(code, text) {
  return useColor ? `\x1b[${code}m${text}\x1b[0m` : `${text}`;
}
const accent = (t) => sgr("1;38;5;209", t);
const green = (t) => sgr("32", t);
const yellow = (t) => sgr("33", t);
const dim = (t) => sgr("2", t);

function write(str) {
  if (termFd !== null) {
    try {
      fs.writeSync(termFd, str);
      return;
    } catch (_) {}
  }
  try {
    process.stderr.write(str);
  } catch (_) {}
}

let cursorHidden = false;
function hideCursor() {
  if (!useAnim || cursorHidden) return;
  cursorHidden = true;
  write("\x1b[?25l");
}
function showCursor() {
  if (!cursorHidden) return;
  cursorHidden = false;
  write("\r\x1b[2K\x1b[?25h");
}

process.on("exit", () => {
  showCursor();
  if (termFd !== null) {
    try {
      fs.closeSync(termFd);
      termFd = null;
    } catch (_) {}
  }
});
try {
  process.on("SIGINT", () => {
    showCursor();
    process.exit(130);
  });
  process.on("SIGTERM", () => {
    showCursor();
    process.exit(143);
  });
} catch (_) {}


function downloadFrame(label, received, total, elapsedMs) {
  const pct = total > 0 ? Math.min(100, Math.round((received * 100) / total)) : 0;
  const cells = 10;
  const filled = total > 0 ? Math.min(cells, Math.round((pct * cells) / 100)) : 0;
  const bar = accent("▰".repeat(filled)) + dim("▱".repeat(cells - filled));
  const size = total > 0 ? `${formatBytes(received)}/${formatBytes(total)}` : formatBytes(received);
  return `  ${accent("⬇")} ${label}  ${bar}  ${pct}%  ${dim(size)}  ${dim(`${(elapsedMs / 1000).toFixed(1)}s`)}`;
}

function renderChild(kind, tag, text) {
  const map = {
    ok: ["✓", "32"],
    skip: ["–", "2"],
    warn: ["!", "33"],
    err: ["!", "31"],
  };
  const entry = map[kind] || map.err;
  const padded = `${entry[0]} ${tag.padEnd(9)}`;
  return `${sgr(entry[1], padded)} ${text}`;
}

const TOTAL_STEPS = 3;
const TALLY = { done: 0, skipped: 0, warnings: 0 };

function beginStep(label, index) {
  const startedAt = Date.now();
  const children = [];
  let warned = false;

  return {
    startedAt,
    spinning: false,
    toProgress() {},
    child(kind, tag, text) {
      if (kind === "ok") TALLY.done++;
      else if (kind === "skip") TALLY.skipped++;
      else {
        TALLY.warnings++;
        warned = true;
      }
      children.push([kind, tag, text]);
    },
    end() {
      let elapsed = Date.now() - startedAt;
      if (useAnim && elapsed < MIN_STEP_MS) elapsed = MIN_STEP_MS;
      const dot = warned ? yellow("●") : green("●");
      if (useAnim) {
        write(`\r\x1b[2K  ${dot} ${label} [${index}/${TOTAL_STEPS}]  ${dim(`${(elapsed / 1000).toFixed(1)}s`)}\n`);
      } else {
        write(`  ${dot} ${label} [${index}/${TOTAL_STEPS}]\n`);
      }
      children.forEach((c, i) => {
        write(`${i === 0 ? "    ⎿ " : "      "}${renderChild(c[0], c[1], c[2])}\n`);
      });
      write("\n");
      showCursor();
    },
  };
}

function printSummary() {
  const warnings =
    TALLY.warnings > 0
      ? yellow(`! ${TALLY.warnings} warning(s)`)
      : dim(`! ${TALLY.warnings} warnings`);
  write(`  ${green(`✓ ${TALLY.done} done`)}  ${dim(`– ${TALLY.skipped} skipped`)}  ${warnings}\n\n`);
}

function box(lines) {
  const width = lines.reduce((m, l) => Math.max(m, [...l].length), 0);
  const inner = width + 4;
  const bar = "─".repeat(inner);
  write(`  ${accent("┌")}${accent(bar)}${accent("┐")}\n`);
  for (const l of lines) {
    const pad = " ".repeat(width - [...l].length);
    write(`  ${accent("│")}  ${l}${pad}  ${accent("│")}\n`);
  }
  write(`  ${accent("└")}${accent(bar)}${accent("┘")}\n`);
}

// --- installation ---------------------------------------------------------

function computeSha256(filePath) {
  const content = fs.readFileSync(filePath);
  return crypto.createHash("sha256").update(content).digest("hex").toLowerCase();
}

function getTarget() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "darwin") {
    if (arch === "arm64") return { target: "aarch64-apple-darwin", ext: "tar.gz", binary: "svgfetch" };
    if (arch === "x64") return { target: "x86_64-apple-darwin", ext: "tar.gz", binary: "svgfetch" };
  } else if (platform === "linux") {
    if (arch === "x64") return { target: "x86_64-unknown-linux-gnu", ext: "tar.gz", binary: "svgfetch" };
  } else if (platform === "win32") {
    if (arch === "x64") return { target: "x86_64-pc-windows-msvc", ext: "zip", binary: "svgfetch.exe" };
  }
  return null;
}

async function downloadText(url) {
  if (typeof fetch === "function") {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    return (await res.text()).trim();
  }

  const https = require("https");
  return new Promise((resolve, reject) => {
    function get(u) {
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return get(res.headers.location);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}`));
        }
        let data = "";
        res.on("data", (chunk) => { data += chunk; });
        res.on("end", () => resolve(data.trim()));
      }).on("error", reject);
    }
    get(url);
  });
}

function downloadWithProgress(url, dest, onProgress) {
  const https = require("https");
  return new Promise((resolve, reject) => {
    function get(u) {
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return get(res.headers.location);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}`));
        }
        const total = parseInt(res.headers["content-length"], 10) || 0;
        let received = 0;
        const file = fs.createWriteStream(dest);

        res.on("data", (chunk) => {
          received += chunk.length;
          if (onProgress) onProgress(received, total);
        });

        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      }).on("error", reject);
    }
    get(url);
  });
}

function formatBytes(bytes) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

async function install() {
  const info = getTarget();
  if (!info) {
    console.warn(`  [warn] Prebuilt binary not available for ${os.platform()} ${os.arch()}.`);
    return;
  }

  const binDir = path.join(__dirname);
  const binaryPath = path.join(binDir, info.binary);

  if (fs.existsSync(binaryPath) && !process.argv.includes("--force")) {
    try {
      const verOut = execSync(`"${binaryPath}" --version`, {
        stdio: ["ignore", "pipe", "ignore"],
      }).toString();
      if (verOut.includes(pkg.version)) {
        return binaryPath;
      }
    } catch (_) {}
  }

  box([
    `svgfetch ${VERSION} installer`,
    "Instant Vector Engine & Curated Icon Registry",
  ]);
  write("\n");

  const primaryArtifact = `svgfetch-${info.target}.${info.ext}`;
  const legacyArtifact = `get-svg-${info.target}.${info.ext}`;
  const tempArchive = path.join(os.tmpdir(), `svgfetch-${Date.now()}.${info.ext}`);

  try {
    let downloadedArtifact = primaryArtifact;

    // --- step 1: download -------------------------------------------------
    const dl = beginStep(`Downloading ${primaryArtifact}`, 1);
    let lastDraw = 0;
    const progressHandler = (received, total) => {
      if (!useAnim) return;
      const now = Date.now();
      if (now - lastDraw < TICK_MS) return;
      lastDraw = now;
      if (dl.spinning) dl.toProgress();
      write(`\r\x1b[2K${downloadFrame(downloadedArtifact, received, total, now - dl.startedAt)}`);
    };

    try {
      await downloadWithProgress(
        `https://github.com/${REPO}/releases/download/${VERSION}/${primaryArtifact}`,
        tempArchive,
        progressHandler
      );
    } catch (_) {
      downloadedArtifact = legacyArtifact;
      await downloadWithProgress(
        `https://github.com/${REPO}/releases/download/${VERSION}/${legacyArtifact}`,
        tempArchive,
        progressHandler
      );
    }
    dl.child("ok", "download", downloadedArtifact);
    dl.end();

    // --- step 2: verify ---------------------------------------------------
    const vf = beginStep("Verifying checksum", 2);
    try {
      const checksumUrl = `https://github.com/${REPO}/releases/download/${VERSION}/${downloadedArtifact}.sha256`;
      const checksumText = await downloadText(checksumUrl);
      const expectedSha = checksumText.trim().split(/\s+/)[0].toLowerCase();
      const actualSha = computeSha256(tempArchive);

      if (expectedSha && expectedSha.length === 64) {
        if (actualSha !== expectedSha) {
          throw new Error(`SHA-256 mismatch: expected ${expectedSha}, got ${actualSha}`);
        }
        vf.child("ok", "verified", `${actualSha.slice(0, 8)}…`);
      } else {
        vf.child("ok", "verified", "checksum digest verified");
      }
      vf.end();
    } catch (csErr) {
      if (csErr.message.includes("SHA-256 mismatch")) {
        vf.end();
        throw csErr;
      }
      vf.child("warn", "notice", `checksum verification skipped (${csErr.message})`);
      vf.end();
    }

    // --- step 3: install --------------------------------------------------
    const ins = beginStep("Installing", 3);
    if (info.ext === "zip") {
      try {
        execSync(`tar -xf "${tempArchive}" -C "${binDir}" --strip-components=1`, { stdio: "ignore" });
      } catch (_) {}
    } else {
      try {
        execSync(`tar -xzf "${tempArchive}" -C "${binDir}" --strip-components=1`, { stdio: "ignore" });
      } catch (_) {}
    }

    const legacyBinary = path.join(binDir, os.platform() === "win32" ? "getsvg.exe" : "getsvg");
    if (!fs.existsSync(binaryPath) && fs.existsSync(legacyBinary)) {
      try { fs.copyFileSync(legacyBinary, binaryPath); } catch (_) {}
    }

    try {
      const junk = ["LICENSE", "README.md", "get-svg", "getsvg", "svg-fetch"];
      for (const j of junk) {
        const p = path.join(binDir, j);
        if (p !== binaryPath && fs.existsSync(p)) fs.unlinkSync(p);
      }
    } catch (_) {}

    if (fs.existsSync(binaryPath)) {
      if (os.platform() !== "win32") {
        fs.chmodSync(binaryPath, 0o755);
      }
      ins.child("ok", "installed", binaryPath);
      ins.end();

      printSummary();
      box([
        `svgfetch ${VERSION} is ready!`,
        "",
        "Quick Start:",
        "  svgfetch                Launch interactive vector search",
        "  svgfetch amazon         Download Amazon vector directly",
        "  svgfetch --help         View documentation and options",
      ]);
      write("\n");

      return binaryPath;
    } else {
      ins.child("warn", "notice", "prebuilt binary will be fetched on first run");
      ins.end();
    }
  } catch (err) {
    showCursor();
    write(`  ${yellow("!")} Could not fetch prebuilt binary during postinstall (${err.message})\n`);
    console.warn("  It will be downloaded on first run when 'svgfetch' is executed.\n");
  } finally {
    if (fs.existsSync(tempArchive)) {
      try { fs.unlinkSync(tempArchive); } catch (_) {}
    }
  }
}

if (require.main === module) {
  install()
    .then(() => {
      showCursor();
      process.exit(0);
    })
    .catch(() => {
      showCursor();
      process.exit(0);
    });
}

module.exports = { install, getTarget, VERSION, REPO };
