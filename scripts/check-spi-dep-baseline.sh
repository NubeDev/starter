#!/usr/bin/env bash
# check-spi-dep-baseline.sh — Smoke test 1 of
# DOCS/tools/scope/SCOPE.md: ensure `starter-spi`'s transitive
# dependency closure matches the committed baseline so a provider
# crate's deps can never sneak in via a re-export or feature toggle.
#
# Usage:
#   scripts/check-spi-dep-baseline.sh             # diff and exit non-zero on mismatch
#   scripts/check-spi-dep-baseline.sh --update    # regenerate the baseline file
#
# The "update" mode is for the rare commit that legitimately changes
# starter-spi's own direct dependencies. Decision D1 forbids using it
# to silence a provider-crate leak.
#
# Normalisation applied before diff (Decision D1's anticipated
# "canonical normalization step"):
#   - LC_ALL=C so sort order is byte-stable across machines.
#   - `(*)` re-display markers from `cargo tree` are stripped.
#   - The workspace-local path on the `starter-spi` self-entry is
#     stripped — CI checkouts and developer worktrees do not share
#     an absolute path, but the bare `starter-spi vX.Y.Z` line is.
#
# Exit codes:
#   0  baseline matches
#   1  baseline drifted (diff printed)
#   2  invocation / environment error

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${REPO_ROOT}/DOCS/tools/scope/starter-spi-deps.baseline.txt"

if [[ ! -f "${BASELINE}" ]]; then
    echo "fatal: baseline file missing at ${BASELINE}" >&2
    exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "fatal: cargo not on PATH" >&2
    exit 2
fi

# Generate the canonical live snapshot. Keep the pipeline in lockstep
# with the header at the top of the baseline file.
snapshot() {
    cargo tree --manifest-path "${REPO_ROOT}/Cargo.toml" \
        -p starter-spi --edges normal --prefix none 2>/dev/null \
        | sed -e 's/ (\*)//' \
              -e "s| (${REPO_ROOT}/[^)]*)||" \
        | sort -u
}

LIVE="$(mktemp)"
BASELINE_BODY="$(mktemp)"
trap 'rm -f "${LIVE}" "${BASELINE_BODY}"' EXIT

snapshot > "${LIVE}"

# Strip the comment header from the baseline before diffing — the
# header documents the generating command, not the dep list.
grep -v -e '^#' -e '^$' "${BASELINE}" | sort -u > "${BASELINE_BODY}"

if [[ "${1-}" == "--update" ]]; then
    {
        # Preserve the existing comment header verbatim.
        awk '/^[^#]/ && NF { exit } { print }' "${BASELINE}"
        snapshot
    } > "${BASELINE}.new"
    mv "${BASELINE}.new" "${BASELINE}"
    echo "updated: ${BASELINE}"
    exit 0
fi

if diff -u "${BASELINE_BODY}" "${LIVE}" > /tmp/spi-baseline-diff 2>&1; then
    echo "starter-spi dep baseline matches."
    exit 0
fi

echo "starter-spi dep baseline drifted:"
echo "---"
cat /tmp/spi-baseline-diff
echo "---"
echo
echo "If this drift is because starter-spi itself gained a new direct"
echo "dependency, rerun this script with --update in the same commit"
echo "that touches starter-spi's Cargo.toml."
echo
echo "If this drift is because a provider crate's deps leaked into"
echo "starter-spi, fix the leak — do not update the baseline. See"
echo "Decision D1 in DOCS/tools/scope/SCOPE.md."
exit 1
