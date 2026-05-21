#!/usr/bin/env bash
# check-sdui-routes-isolation.sh — Smoke test 10 of
# DOCS/frontend/sdui/SCOPE.md § Smoke tests.
#
# `starter-server` MUST NOT depend on `starter-sdui-routes`.
# Per M6 / D4 in SCOPE.md and DIVERGENCE.md: Cargo features cannot
# prevent transitive compilation of `starter-ui-ir` /
# `starter-ui-bindings`; the only honest opt-out is a separate
# crate the consumer pulls in via their own Cargo.toml. If
# `starter-server` ever depends on `starter-sdui-routes`, every
# starter-server consumer pays the SDUI dep-graph cost whether they
# adopt SDUI or not — the consumer-opt-in claim breaks.
#
# This script checks the transitive dep closure (not just the
# Cargo.toml line) — a re-export through another crate would
# defeat a regex check.

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "fatal: cargo not on PATH" >&2
    exit 2
fi

# All-features view, because a feature flag MUST NOT smuggle the
# dep in either — per M6 a feature on starter-server that pulls
# starter-sdui-routes still means "compiling starter-server may
# compile the SDUI graph", which is exactly what consumer-opt-in
# is supposed to prevent.
TREE="$(cargo tree --manifest-path "${REPO_ROOT}/Cargo.toml" \
    -p starter-server --edges normal --all-features --prefix none 2>/dev/null \
    | sed 's/ (\*)//' \
    | awk '{print $1}' \
    | sort -u)"

DENY=(
    starter-sdui-routes
    starter-ui-ir
    starter-ui-bindings
    starter-ui-builder
)

FAIL=0
for crate in "${DENY[@]}"; do
    if echo "$TREE" | grep -Fxq "$crate"; then
        if [[ $FAIL -eq 0 ]]; then
            echo "SDUI consumer-opt-in gate FAILED:" >&2
            echo "  starter-server's transitive deps contain SDUI crates," >&2
            echo "  which breaks the consumer-opt-in claim from M6 / D4:" >&2
        fi
        echo "    $crate" >&2
        FAIL=1
    fi
done

if [[ $FAIL -ne 0 ]]; then
    exit 1
fi

echo "Smoke 10 passed: starter-server has no SDUI crates in its transitive normal closure."
