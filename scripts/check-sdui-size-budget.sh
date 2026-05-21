#!/usr/bin/env bash
# check-sdui-size-budget.sh — Stage 12 size-budget CI gate from
# DOCS/frontend/sdui/SCOPE.md.
#
# Two budgets, both red lines (build fails on overflow, no soft
# warning tier):
#
#   1. `packages/starter-sdui-react/src/Renderer.tsx` ≤ 800 LoC.
#      The single dispatcher file. Past this size, the renderer is
#      growing logic instead of dispatching; that's an R3 / R9
#      smell waiting to happen.
#
#   2. `packages/starter-sdui-react/src/components/*.tsx`,
#      excluding `*.test.tsx`, total ≤ 4000 LoC.
#      The component implementations as a whole. The SCOPE budgets
#      3000 LoC as the target and 4000 as the red line; this gate
#      enforces the red line. Test files are excluded — tests grow
#      with the test suite, not the renderer.

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RENDERER="${REPO_ROOT}/packages/starter-sdui-react/src/Renderer.tsx"
COMPONENTS_DIR="${REPO_ROOT}/packages/starter-sdui-react/src/components"

RENDERER_LIMIT=800
COMPONENTS_LIMIT=4000

if [[ ! -f "$RENDERER" ]]; then
    echo "fatal: $RENDERER missing" >&2
    exit 2
fi
if [[ ! -d "$COMPONENTS_DIR" ]]; then
    echo "fatal: $COMPONENTS_DIR missing" >&2
    exit 2
fi

renderer_lines=$(wc -l < "$RENDERER")
components_lines=$(find "$COMPONENTS_DIR" -maxdepth 1 -type f -name '*.tsx' \
    ! -name '*.test.tsx' -print0 \
    | xargs -0 wc -l \
    | awk '/total$/ {print $1; found=1} END {if (!found) print 0}' \
    | tail -1)
# `wc -l` prints no "total" line when given a single file, so fall back.
if [[ -z "$components_lines" || "$components_lines" == "0" ]]; then
    components_lines=$(find "$COMPONENTS_DIR" -maxdepth 1 -type f -name '*.tsx' \
        ! -name '*.test.tsx' -exec wc -l {} + \
        | awk '{sum += $1} END {print sum+0}')
fi

FAIL=0
if (( renderer_lines > RENDERER_LIMIT )); then
    echo "SIZE BUDGET FAILED: Renderer.tsx is ${renderer_lines} lines (limit ${RENDERER_LIMIT})." >&2
    FAIL=1
fi
if (( components_lines > COMPONENTS_LIMIT )); then
    echo "SIZE BUDGET FAILED: total components ${components_lines} lines (limit ${COMPONENTS_LIMIT})." >&2
    FAIL=1
fi

if [[ $FAIL -eq 0 ]]; then
    printf 'Size budget OK: Renderer.tsx=%d/%d, components=%d/%d.\n' \
        "$renderer_lines" "$RENDERER_LIMIT" \
        "$components_lines" "$COMPONENTS_LIMIT"
fi
exit $FAIL
