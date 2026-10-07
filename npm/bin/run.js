#!/usr/bin/env node

const path = require('path');
const fs = require('fs');
const os = require('os');
const { spawn } = require('child_process');
const { getTarget, install } = require('./install');

async function main() {
  const info = getTarget();
  if (!info) {
    console.error(`[svgfetch] Error: Unsupported platform/architecture: ${os.platform()} ${os.arch()}`);
    console.error(`[svgfetch] Supported: macOS (Apple Silicon/Intel), Linux (x64), Windows (x64)`);
    process.exit(1);
  }

  const binaryPath = path.join(__dirname, info.binary);

  if (!fs.existsSync(binaryPath)) {
    try {
      await install();
    } catch (err) {
      console.error(`[svgfetch] Error installing binary:`, err.message);
      process.exit(1);
    }
  }

  if (!fs.existsSync(binaryPath)) {
    console.error(`[svgfetch] Binary not found at ${binaryPath}`);
    process.exit(1);
  }

  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    windowsHide: false
  });

  child.on('error', (err) => {
    console.error(`[svgfetch] Execution error:`, err);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code ?? 0);
    }
  });
}

main().catch((err) => {
  console.error(`[svgfetch] Fatal:`, err);
  process.exit(1);
});
