#!/usr/bin/env bash
# Verifies drift web assets exist and the resolved drift version
# falls within the declared compat band.
set -euo pipefail

cd "$(dirname "$0")/.."

# 1. Check web assets exist
for f in web/sqlite3.wasm web/drift_worker.dart.js; do
  if [[ ! -f "$f" ]]; then
    echo "FAIL: $f not found. Download from the drift release matching driftAssetsReleaseTag."
    exit 1
  fi
done

# 2. Parse resolved drift version from pubspec.lock
DRIFT_VERSION=$(grep -A10 '^\s*drift:' pubspec.lock | grep 'version:' | head -1 | sed 's/.*version: *"\{0,1\}\([^"]*\)"\{0,1\}/\1/' | tr -d ' ')
if [[ -z "$DRIFT_VERSION" ]]; then
  echo "FAIL: could not parse drift version from pubspec.lock"
  exit 1
fi

# 3. Parse compat range from source
COMPAT_FILE="lib/core/storage/_drift_assets_version.dart"
MIN=$(grep "driftAssetsCompatRange" "$COMPAT_FILE" | grep -oP "min: '\K[0-9]+\.[0-9]+\.[0-9]+")
MAX=$(grep "driftAssetsCompatRange" "$COMPAT_FILE" | grep -oP "max: '\K[0-9]+\.[0-9]+\.[0-9]+")

# 4. Version comparison (lexicographic works for semver with same digit counts)
version_lte() {
  printf '%s\n%s' "$1" "$2" | sort -V | head -1 | grep -qx "$1"
}

if version_lte "$MIN" "$DRIFT_VERSION" && version_lte "$DRIFT_VERSION" "$MAX"; then
  echo "OK: drift $DRIFT_VERSION is within compat band [$MIN, $MAX]"
else
  echo "FAIL: drift $DRIFT_VERSION is outside compat band [$MIN, $MAX]"
  echo "Either widen driftAssetsCompatRange or refresh web/ assets."
  exit 1
fi
