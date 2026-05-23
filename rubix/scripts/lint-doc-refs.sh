#!/usr/bin/env bash
# lint-doc-refs.sh — enforce the rubix doc-tier rule.
#
# Per HOW-TO-CODE.md §0a and NEW-SESSION.md §2, rubix Rust source
# files may reference `docs/design/<area>/README.md` only. They MUST
# NOT reference SCOPE.md, HOW-TO-CODE.md, NEW-SESSION.md,
# FILE-LAYOUT.md, docs/scope/, or docs/sessions/. ADR references are
# permitted but flagged as warnings (rare exceptions).
#
# Exit codes:
#   0  clean
#   1  one or more forbidden references found
#
# Run from the repository root:   ./rubix/scripts/lint-doc-refs.sh
# Or via mani:                    mani run lint-doc-refs

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC_GLOB="$ROOT/rubix/crates"

if [[ ! -d "$SRC_GLOB" ]]; then
    echo "lint-doc-refs: $SRC_GLOB does not exist" >&2
    exit 1
fi

FORBIDDEN_REGEX='SCOPE\.md|HOW-TO-CODE\.md|NEW-SESSION\.md|FILE-LAYOUT\.md|docs/scope/|docs/sessions/'
WARN_REGEX='docs/adr/'

failures=0

# Forbidden references — these fail the lint.
while IFS= read -r hit; do
    if [[ -n "$hit" ]]; then
        echo "FORBIDDEN doc-tier reference: $hit"
        failures=$((failures + 1))
    fi
done < <(grep -RInE "$FORBIDDEN_REGEX" --include='*.rs' "$SRC_GLOB" || true)

# ADR references — warn only.
while IFS= read -r hit; do
    if [[ -n "$hit" ]]; then
        echo "WARN ADR reference (review for necessity): $hit"
    fi
done < <(grep -RInE "$WARN_REGEX" --include='*.rs' "$SRC_GLOB" || true)

if (( failures > 0 )); then
    echo ""
    echo "lint-doc-refs: $failures forbidden reference(s) found." >&2
    echo "Per HOW-TO-CODE.md §0a, code may only link docs/design/<area>/README.md." >&2
    exit 1
fi

echo "lint-doc-refs: clean."
exit 0
