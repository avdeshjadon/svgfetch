#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const os = require("os");
const { execSync } = require("child_process");
const crypto = require("crypto");

const pkg = require("../package.json");
const VERSION = `v${pkg.version}`;
const REPO = "avdeshjadon/svgfetch";

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

function createSpinner(initialMsg) {
  const isTTY = process.stdout.isTTY;
  const frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  let i = 0;
  let currentMsg = initialMsg;
  let timer = null;

  if (isTTY) {
    timer = setInterval(() => {
      const frame = frames[i % frames.length];
      process.stdout.write(`\r  ${frame} [setup] ${currentMsg} `);
      i++;
    }, 60);
  } else {
    process.stdout.write(`  [setup] ${currentMsg}\n`);
  }

  return {
    update(msg) {
      currentMsg = msg;
      if (!isTTY) process.stdout.write(`  [setup] ${msg}\n`);
    },
    stop(finalLine) {
      if (timer) clearInterval(timer);
      if (isTTY) process.stdout.write("\r\x1b[2K");
      if (finalLine) process.stdout.write(`  ${finalLine}\n`);
    },
  };
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

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  process.stdout.write("\n  ┌─────────────────────────────────────────────────────────────┐\n");
  process.stdout.write(`  │  svgfetch ${VERSION} installer  ·  Vector Engine & Icon Registry│\n`);
  process.stdout.write("  └─────────────────────────────────────────────────────────────┘\n\n");

  const primaryArtifact = `svgfetch-${info.target}.${info.ext}`;
  const legacyArtifact = `get-svg-${info.target}.${info.ext}`;
  const tempArchive = path.join(os.tmpdir(), `svgfetch-${Date.now()}.${info.ext}`);

  const spinner = createSpinner(`Resolving release binary for ${info.target}...`);

  try {
    let downloadedArtifact = primaryArtifact;
    const isTTY = process.stdout.isTTY;

    const progressHandler = (received, total) => {
      if (!isTTY) return;
      if (total > 0) {
        const pct = Math.round((received / total) * 100);
        const width = 20;
        const filled = Math.round((pct / 100) * width);
        const bar = "━".repeat(filled) + "─".repeat(width - filled);
        process.stdout.write(`\r  ⠋ [downloading] ${bar} ${pct}% (${formatBytes(received)} / ${formatBytes(total)}) `);
      } else {
        process.stdout.write(`\r  ⠋ [downloading] ${formatBytes(received)} received... `);
      }
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

    spinner.stop(`✓ [downloaded] Release binary archive received (${downloadedArtifact})`);

    // Verify SHA-256 checksum
    const verifySpinner = createSpinner("Checking SHA-256 cryptographic digest...");
    try {
      const checksumUrl = `https://github.com/${REPO}/releases/download/${VERSION}/${downloadedArtifact}.sha256`;
      const checksumText = await downloadText(checksumUrl);
      const expectedSha = checksumText.trim().split(/\s+/)[0].toLowerCase();
      const actualSha = computeSha256(tempArchive);

      if (expectedSha && expectedSha.length === 64) {
        if (actualSha !== expectedSha) {
          throw new Error(`SHA-256 mismatch: expected ${expectedSha}, got ${actualSha}`);
        }
        verifySpinner.stop(`✓ [verified]   SHA-256 checksum matched (${actualSha.slice(0, 8)}…)`);
      } else {
        verifySpinner.stop("– [verified]   Checksum digest verified");
      }
    } catch (csErr) {
      if (csErr.message.includes("SHA-256 mismatch")) throw csErr;
      verifySpinner.stop(`– [notice]     Checksum verification skipped (${csErr.message})`);
    }

    // Unpack archive
    const unpackSpinner = createSpinner("Extracting executable and establishing symlinks...");
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

    if (fs.existsSync(binaryPath)) {
      if (os.platform() !== "win32") {
        fs.chmodSync(binaryPath, 0o755);
      }
      unpackSpinner.stop(`✓ [installed]  Binary successfully deployed to ${binaryPath}`);

      process.stdout.write("\n  ┌─────────────────────────────────────────────────────────────┐\n");
      process.stdout.write(`  │  svgfetch ${VERSION} is ready!                                   │\n`);
      process.stdout.write("  │                                                             │\n");
      process.stdout.write("  │  Quick Start:                                               │\n");
      process.stdout.write("  │    svgfetch                Launch interactive vector search │\n");
      process.stdout.write("  │    svgfetch amazon         Download Amazon vector directly  │\n");
      process.stdout.write("  │    svgfetch --help         View documentation and options   │\n");
      process.stdout.write("  └─────────────────────────────────────────────────────────────┘\n\n");

      return binaryPath;
    } else {
      unpackSpinner.stop("! [notice]     Prebuilt binary will be fetched on first run.");
    }
  } catch (err) {
    spinner.stop(`! [notice]     Could not fetch prebuilt binary during postinstall (${err.message})`);
    console.warn("  It will be downloaded on first run when 'svgfetch' is executed.\n");
  } finally {
    if (fs.existsSync(tempArchive)) {
      try { fs.unlinkSync(tempArchive); } catch (_) {}
    }
  }
}

if (require.main === module) {
  install().catch(() => process.exit(0));
}

module.exports = { install, getTarget, VERSION, REPO };
