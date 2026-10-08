#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

const pkg = require('../package.json');
const VERSION = `v${pkg.version}`;
const REPO = 'avdeshjadon/svgfetch';

function getTarget() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === 'darwin') {
    if (arch === 'arm64') return { target: 'aarch64-apple-darwin', ext: 'tar.gz', binary: 'svgfetch' };
    if (arch === 'x64') return { target: 'x86_64-apple-darwin', ext: 'tar.gz', binary: 'svgfetch' };
  } else if (platform === 'linux') {
    if (arch === 'x64') return { target: 'x86_64-unknown-linux-gnu', ext: 'tar.gz', binary: 'svgfetch' };
  } else if (platform === 'win32') {
    if (arch === 'x64') return { target: 'x86_64-pc-windows-msvc', ext: 'zip', binary: 'svgfetch.exe' };
  }
  return null;
}

async function download(url, dest) {
  if (typeof fetch === 'function') {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    const buffer = Buffer.from(await res.arrayBuffer());
    fs.writeFileSync(dest, buffer);
    return;
  }

  const https = require('https');
  return new Promise((resolve, reject) => {
    function get(u) {
      https.get(u, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return get(res.headers.location);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}`));
        }
        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
      }).on('error', reject);
    }
    get(url);
  });
}

async function install() {
  const info = getTarget();
  if (!info) {
    console.warn(`[svgfetch] Warning: Prebuilt binary not available for ${os.platform()} ${os.arch()}.`);
    return;
  }

  const binDir = path.join(__dirname);
  const binaryPath = path.join(binDir, info.binary);

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  const primaryArtifact = `svgfetch-${info.target}.${info.ext}`;
  const legacyArtifact = `get-svg-${info.target}.${info.ext}`;
  const tempArchive = path.join(os.tmpdir(), `svgfetch-${Date.now()}.${info.ext}`);

  try {
    process.stdout.write(`[svgfetch] Downloading prebuilt binary…\n`);
    try {
      await download(`https://github.com/${REPO}/releases/download/${VERSION}/${primaryArtifact}`, tempArchive);
    } catch (_) {
      await download(`https://github.com/${REPO}/releases/download/${VERSION}/${legacyArtifact}`, tempArchive);
    }

    if (info.ext === 'zip') {
      try {
        execSync(`tar -xf "${tempArchive}" -C "${binDir}" --strip-components=1`, { stdio: 'ignore' });
      } catch (_) {}
    } else {
      try {
        execSync(`tar -xzf "${tempArchive}" -C "${binDir}" --strip-components=1`, { stdio: 'ignore' });
      } catch (_) {}
    }

    // Handle legacy archive naming (where binary inside was getsvg or get-svg)
    const legacyBinary = path.join(binDir, os.platform() === 'win32' ? 'getsvg.exe' : 'getsvg');
    if (!fs.existsSync(binaryPath) && fs.existsSync(legacyBinary)) {
      try {
        fs.copyFileSync(legacyBinary, binaryPath);
      } catch (_) {}
    }

    if (fs.existsSync(binaryPath)) {
      if (os.platform() !== 'win32') {
        fs.chmodSync(binaryPath, 0o755);
      }
      process.stdout.write(`[svgfetch] Successfully installed to ${binaryPath}\n`);
      return binaryPath;
    }
  } catch (err) {
    console.warn(`[svgfetch] Notice: Could not download prebuilt binary during postinstall (${err.message}).`);
    console.warn(`[svgfetch] It will be downloaded on first run when 'svgfetch' or 'npx svgfetch' is executed.`);
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
