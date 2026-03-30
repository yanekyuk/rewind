#!/usr/bin/env bash
# Download DepotDownloader self-contained binaries.
# Places them in src-tauri/binaries/ with Tauri's sidecar naming convention.
#
# Usage: ./scripts/download-sidecar.sh [--current] [version]
#   --current: only download the binary for the current host platform
#   version:   DepotDownloader release tag (default: latest)
#
# Requirements: curl, unzip

set -euo pipefail

REPO="SteamRE/DepotDownloader"
BINARIES_DIR="src-tauri/binaries"
CURRENT_ONLY=false
VERSION=""

for arg in "$@"; do
  case "$arg" in
    --current) CURRENT_ONLY=true ;;
    *) VERSION="$arg" ;;
  esac
done

# Resolve latest version if not specified
if [ -z "$VERSION" ]; then
  echo "Fetching latest release tag..."
  VERSION=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
  if [ -z "$VERSION" ]; then
    echo "Error: Could not determine latest release version." >&2
    exit 1
  fi
fi

echo "DepotDownloader version: ${VERSION}"

# Strip leading 'DepotDownloader_' or 'v' prefix for URL construction
URL_VERSION="${VERSION}"

BASE_URL="https://github.com/${REPO}/releases/download/${URL_VERSION}"

# Platform mappings: archive name -> target triple
declare -A PLATFORMS=(
  ["DepotDownloader-linux-x64.zip"]="x86_64-unknown-linux-gnu"
  ["DepotDownloader-macos-x64.zip"]="x86_64-apple-darwin"
  ["DepotDownloader-macos-arm64.zip"]="aarch64-apple-darwin"
  ["DepotDownloader-windows-x64.zip"]="x86_64-pc-windows-msvc"
)

# If --current, filter to only the host platform's archive
if [ "$CURRENT_ONLY" = true ]; then
  HOST=$(rustc -vV 2>/dev/null | grep '^host:' | cut -d' ' -f2)
  if [ -z "$HOST" ]; then
    echo "Error: could not determine host target triple (is rustc installed?)" >&2
    exit 1
  fi
  echo "Host target: ${HOST}"
  filtered=()
  for archive in "${!PLATFORMS[@]}"; do
    if [ "${PLATFORMS[$archive]}" = "$HOST" ]; then
      filtered+=("$archive")
    fi
  done
  if [ ${#filtered[@]} -eq 0 ]; then
    echo "Error: no binary mapping found for host '${HOST}'" >&2
    exit 1
  fi
  # Rebuild PLATFORMS with only the matching entry
  declare -A PLATFORMS_FILTERED
  for a in "${filtered[@]}"; do
    PLATFORMS_FILTERED["$a"]="${PLATFORMS[$a]}"
  done
  unset PLATFORMS
  declare -A PLATFORMS
  for a in "${!PLATFORMS_FILTERED[@]}"; do
    PLATFORMS["$a"]="${PLATFORMS_FILTERED[$a]}"
  done
fi

# Binary name inside archive -> expected name (for Windows .exe)
declare -A EXTENSIONS=(
  ["x86_64-unknown-linux-gnu"]=""
  ["x86_64-apple-darwin"]=""
  ["aarch64-apple-darwin"]=""
  ["x86_64-pc-windows-msvc"]=".exe"
)

mkdir -p "$BINARIES_DIR"
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

for archive in "${!PLATFORMS[@]}"; do
  target="${PLATFORMS[$archive]}"
  ext="${EXTENSIONS[$target]}"
  url="${BASE_URL}/${archive}"
  dest="${BINARIES_DIR}/DepotDownloader-${target}${ext}"

  echo ""
  echo "Downloading ${archive}..."
  if ! curl -fSL -o "${TMPDIR}/${archive}" "$url"; then
    echo "Error: Failed to download ${url}" >&2
    echo "Check that version '${VERSION}' exists and has asset '${archive}'." >&2
    exit 1
  fi

  echo "Extracting to ${dest}..."
  unzip -o -j "${TMPDIR}/${archive}" -d "${TMPDIR}/${target}" > /dev/null

  # Find the DepotDownloader binary in the extracted files
  if [ -n "$ext" ]; then
    src=$(find "${TMPDIR}/${target}" -name "DepotDownloader${ext}" -type f | head -1)
  else
    src=$(find "${TMPDIR}/${target}" -name "DepotDownloader" -type f | head -1)
  fi

  if [ -z "$src" ]; then
    echo "Error: Could not find DepotDownloader binary in ${archive}" >&2
    exit 1
  fi

  cp "$src" "$dest"
  chmod +x "$dest"
  echo "Installed: ${dest}"
done

echo ""
echo "All DepotDownloader binaries installed to ${BINARIES_DIR}/"
echo "Version: ${VERSION}"
