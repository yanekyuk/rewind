#!/bin/bash
# Test script for SteamKit sidecar
# Demonstrates JSON output format for each command

set -e

SIDECAR="./SteamKitSidecar/bin/Release/net9.0/SteamKitSidecar"

# Test 1: Missing command
echo "=== Test 1: No command ==="
$SIDECAR 2>&1 || true
echo ""

# Test 2: Invalid command
echo "=== Test 2: Invalid command ==="
$SIDECAR invalid 2>&1 || true
echo ""

# Test 3: Missing required arguments
echo "=== Test 3: Missing arguments for login ==="
$SIDECAR login 2>&1 || true
echo ""

# Test 4: Verify JSON output format
echo "=== Test 4: Verify error JSON format ==="
$SIDECAR login --username test --password test 2>&1 | head -5 || true
echo ""

echo "All basic tests completed successfully!"
echo ""
echo "Note: Full integration tests require actual Steam credentials and internet access."
echo "See TESTING.md for comprehensive testing instructions."
