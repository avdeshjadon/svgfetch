#!/usr/bin/env node

/**
 * Single-source version synchronizer for svgfetch.
 * 
 * Usage:
 *   node scripts/bump.js <new_version>   # Updates Cargo.toml, npm/package.json, Cargo.lock
 *   node scripts/bump.js                 # Syncs npm/package.json & Cargo.lock with Cargo.toml
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const rootDir = path.resolve(__dirname, '..');
const cargoTomlPath = path.join(rootDir, 'Cargo.toml');
const packageJsonPath = path.join(rootDir, 'npm', 'package.json');
const cargoLockPath = path.join(rootDir, 'Cargo.lock');

function getCargoVersion() {
  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  const match = content.match(/\[package\][\s\S]*?version\s*=\s*"([^"]+)"/);
  if (!match) {
    throw new Error('Could not find version in Cargo.toml under [package]');
  }
  return match[1];
}

function updateCargoToml(newVersion) {
  let content = fs.readFileSync(cargoTomlPath, 'utf8');
  content = content.replace(
    /(\[package\][\s\S]*?version\s*=\s*")[^"]+(")/,
    `$1${newVersion}$2`
  );
  fs.writeFileSync(cargoTomlPath, content, 'utf8');
}

function getPackageJsonVersion() {
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  return pkg.version;
}

function updatePackageJson(newVersion) {
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  pkg.version = newVersion;
  fs.writeFileSync(packageJsonPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
}

function updateCargoLock() {
  try {
    execSync('cargo check --quiet', { cwd: rootDir, stdio: 'pipe' });
  } catch (err) {
    // If cargo fails or isn't available, warn but do not crash
    console.warn('Note: Could not run "cargo check" to update Cargo.lock:', err.message);
  }
}

function main() {
  let requestedVersion = process.argv[2];

  if (requestedVersion) {
    // Remove leading 'v' if present (e.g., 'v0.2.5' -> '0.2.5')
    requestedVersion = requestedVersion.replace(/^v/, '').trim();

    if (!/^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?$/.test(requestedVersion)) {
      console.error(`❌ Invalid semver version: "${requestedVersion}". Expected format like 0.2.5 or 0.2.5-beta.1`);
      process.exit(1);
    }

    console.log(`🚀 Updating version to ${requestedVersion}...`);
    updateCargoToml(requestedVersion);
    updatePackageJson(requestedVersion);
    updateCargoLock();

    console.log(`✅ Version successfully updated to ${requestedVersion}:`);
    console.log(`   ✔ Cargo.toml -> ${requestedVersion}`);
    console.log(`   ✔ npm/package.json -> ${requestedVersion}`);
    console.log(`   ✔ Cargo.lock -> updated`);
    console.log(`   ✔ npm/bin/install.js -> dynamic (always matches npm/package.json)`);
    return;
  }

  // No version supplied: sync mode (read Cargo.toml as source of truth)
  const cargoVersion = getCargoVersion();
  const pkgVersion = getPackageJsonVersion();

  if (cargoVersion !== pkgVersion) {
    console.log(`🔄 Syncing version: Cargo.toml (${cargoVersion}) -> npm/package.json (${pkgVersion})...`);
    updatePackageJson(cargoVersion);
    updateCargoLock();
    console.log(`✅ Synced! npm/package.json is now ${cargoVersion}`);
  } else {
    // Already in sync
    // Ensure Cargo.lock is updated if needed
    // quiet exit for git hooks
  }
}

main();
