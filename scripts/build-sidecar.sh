#!/usr/bin/env bash
# Build the SteamKit-based sidecar (.NET project).
# Places the compiled binary in src-tauri/target/debug/binaries/ with Tauri's sidecar naming convention.
#
# Usage: ./scripts/build-sidecar.sh [--release]
#   --release: build in Release mode instead of Debug
#
# Requirements: dotnet CLI

set -euo pipefail

SIDECAR_PROJECT_DIR="sidecar/SteamKitSidecar"
SIDECAR_OUTPUT_DIR="src-tauri/target/debug/binaries"
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

echo "Building SteamKit sidecar (${BUILD_CONFIG} configuration)..."

# Build the .NET project
cd "$SIDECAR_PROJECT_DIR"
dotnet publish -c "$BUILD_CONFIG" -o "../../${SIDECAR_OUTPUT_DIR}" --self-contained=false

cd - > /dev/null

# Ensure the output directory exists and the binary is executable
mkdir -p "$SIDECAR_OUTPUT_DIR"

# Find and set executable permission on the binary
# On Windows, it will be SteamKitSidecar.exe; on Unix, just SteamKitSidecar
if [ -f "${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}.exe" ]; then
  chmod +x "${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}.exe"
  echo "Built: ${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}.exe"
elif [ -f "${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}" ]; then
  chmod +x "${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}"
  echo "Built: ${SIDECAR_OUTPUT_DIR}/${SIDECAR_NAME}"
else
  echo "Error: Could not find built sidecar binary" >&2
  exit 1
fi

echo "SteamKit sidecar built successfully"
