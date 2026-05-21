#!/usr/bin/env bash
# check-sdui-ir-deps.sh — R1 (DOCS/frontend/sdui/SCOPE.md):
# `starter-ui-ir` is zero-I/O. Its transitive normal dep closure
# must not contain any I/O / runtime / HTTP machinery; a feature
# flag on a transitive crate that pulls those in is a regression
# of the same kind R1 prohibits.
#
# This script greps `cargo tree -p starter-ui-ir --edges normal`
# against a fixed denylist. It is the transitive-graph form of the
# rule, NOT a Cargo.toml regex (which only checks direct deps).
#
# Denylist (per the SCOPE.md § R1 paragraph and the Stage 12
# brief):
#   axum, axum-core, reqwest, hyper, tokio, tokio-util,
#   tower, tower-http, h2, http-body
#
# Exit codes:
#   0 — clean
#   1 — at least one denied crate in the transitive closure
#   2 — environment error

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "fatal: cargo not on PATH" >&2
    exit 2
fi

DENY=(
    axum
    axum-core
    reqwest
    hyper
    tokio
    tokio-util
    tower
    tower-http
    h2
    http-body
)

TREE="$(cargo tree --manifest-path "${REPO_ROOT}/Cargo.toml" \
    -p starter-ui-ir --edges normal --prefix none 2>/dev/null \
    | sed 's/ (\*)//' \
    | awk '{print $1}' \
    | sort -u)"

FAIL=0
for crate in "${DENY[@]}"; do
    if echo "$TREE" | grep -Fxq "$crate"; then
        if [[ $FAIL -eq 0 ]]; then
            echo "R1 dep-tree gate FAILED for starter-ui-ir:" >&2
            echo "  the following I/O / runtime crates appear in" >&2
            echo "  the transitive normal-dep closure:" >&2
        fi
        echo "    $crate" >&2
        FAIL=1
    fi
done

if [[ $FAIL -ne 0 ]]; then
    echo "" >&2
    echo "  full tree:" >&2
    cargo tree --manifest-path "${REPO_ROOT}/Cargo.toml" \
        -p starter-ui-ir --edges normal 2>/dev/null \
        | sed 's/^/    /' >&2
    exit 1
fi

echo "R1 dep-tree check passed: starter-ui-ir's transitive normal closure is I/O-free."
