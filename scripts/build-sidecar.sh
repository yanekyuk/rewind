#!/usr/bin/env bash
# Build the SteamKit-based sidecar (.NET project).
# Places the compiled binary in both:
#   - src-tauri/binaries/ (for Tauri build-time bundling)
#   - src-tauri/target/debug/binaries/ (for Tauri dev-mode runtime)
#
# Usage: ./scripts/build-sidecar.sh [--release]
#   --release: build in Release mode instead of Debug
#
# Requirements: dotnet CLI, rustc (for target triple detection)

set -euo pipefail

SIDECAR_PROJECT_DIR="sidecar/SteamKitSidecar"
BUILD_CONFIG="Debug"
SIDECAR_NAME="SteamKitSidecar"

# Parse arguments
for arg in "$@"; do
  case "$arg" in
    --release) BUILD_CONFIG="Release" ;;
  esac
done

# Verify dotnet is installed
if ! command -v dotnet &> /dev/null; then
  echo "Error: dotnet CLI is not installed. Please install .NET SDK." >&2
  exit 1
fi

# Verify sidecar project exists
if [ ! -d "$SIDECAR_PROJECT_DIR" ]; then
  echo "Error: Sidecar project directory not found at ${SIDECAR_PROJECT_DIR}" >&2
  exit 1
fi

# Determine target triple for Tauri sidecar naming convention
TARGET_TRIPLE="${TARGET_TRIPLE:-$(rustc -vV | sed -n 's|host: ||p')}"

# Tauri dev-mode resolves sidecars relative to the Rust binary in target/
DEV_OUTPUT_DIR="src-tauri/target/debug/binaries"
# Tauri build-time copies sidecars from src-tauri/binaries/
BUNDLE_OUTPUT_DIR="src-tauri/binaries"

echo "Building SteamKit sidecar (${BUILD_CONFIG} configuration)..."

# Build and publish to the dev output directory (where Tauri dev finds it)
mkdir -p "$DEV_OUTPUT_DIR"

dotnet publish "$SIDECAR_PROJECT_DIR" -c "$BUILD_CONFIG" -o "$DEV_OUTPUT_DIR" --self-contained=false

# Rename binary with target triple suffix in dev dir
if [ -f "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}.exe" ]; then
  cp "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}.exe" "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}.exe"
  echo "Built (dev): ${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}.exe"
elif [ -f "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}" ]; then
  cp "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}" "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}"
  chmod +x "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}"
  echo "Built (dev): ${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}"
else
  echo "Error: Could not find built sidecar binary" >&2
  exit 1
fi

# Also copy the triple-suffixed binary + runtime deps to bundle dir (for tauri build)
mkdir -p "$BUNDLE_OUTPUT_DIR"
cp "${DEV_OUTPUT_DIR}"/*.dll "${BUNDLE_OUTPUT_DIR}/" 2>/dev/null || true
cp "${DEV_OUTPUT_DIR}"/*.json "${BUNDLE_OUTPUT_DIR}/" 2>/dev/null || true
if [ -f "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}.exe" ]; then
  cp "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}.exe" "${BUNDLE_OUTPUT_DIR}/"
else
  cp "${DEV_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}" "${BUNDLE_OUTPUT_DIR}/"
fi

echo "Built (bundle): ${BUNDLE_OUTPUT_DIR}/${SIDECAR_NAME}-${TARGET_TRIPLE}"
echo "SteamKit sidecar built successfully"
