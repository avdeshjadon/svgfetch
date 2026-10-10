#!/usr/bin/env bash
# Syncs the bundled curated manifest from the svgfetch-icons repository.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_FILE="${ROOT_DIR}/data/curated_manifest.json"

MANIFEST_URL="https://cdn.jsdelivr.net/gh/avdeshjadon/svgfetch-icons@main/manifest.json"
RAW_URL="https://raw.githubusercontent.com/avdeshjadon/svgfetch-icons/main/manifest.json"

echo "[sync] Fetching latest manifest from svgfetch-icons..."
mkdir -p "${ROOT_DIR}/data"

if curl -sSL --fail "${MANIFEST_URL}" -o "${TARGET_FILE}.tmp"; then
    mv "${TARGET_FILE}.tmp" "${TARGET_FILE}"
    COUNT=$(grep -o '"shortname"' "${TARGET_FILE}" | wc -l | tr -d ' ')
    echo "[sync] Successfully updated ${TARGET_FILE} (${COUNT} brands synced from CDN)"
elif curl -sSL --fail "${RAW_URL}" -o "${TARGET_FILE}.tmp"; then
    mv "${TARGET_FILE}.tmp" "${TARGET_FILE}"
    COUNT=$(grep -o '"shortname"' "${TARGET_FILE}" | wc -l | tr -d ' ')
    echo "[sync] Successfully updated ${TARGET_FILE} (${COUNT} brands synced from GitHub raw)"
else
    rm -f "${TARGET_FILE}.tmp"
    echo "[error] Failed to download manifest from both CDN and GitHub" >&2
    exit 1
fi
