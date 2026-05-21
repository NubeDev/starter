#!/usr/bin/env bash
# check-sdui-domain-leak.sh — Smoke test 1 of
# DOCS/frontend/sdui/SCOPE.md § R3.
#
# Scans the renderer-side and contract-side SDUI crates for any
# identifier (length ≥ 4) that is not listed in that crate's
# per-crate `words.txt` allowlist. The allowlist is the contract,
# the keyword denylist is defence-in-depth.
#
# Per the SCOPE: domain leak prevention by structural enforcement,
# not a fixed denylist of "building|device|alarm|..." which would
# pass silently for the next consumer whose vocabulary is
# "vehicle|policy|claim|...". The allowlist is the contract: a new
# word in the source has to be added to `words.txt` in the same PR
# and the PR description must name the framework concept it
# represents.
#
# Scope per SCOPE.md § R3:
#   - `crates/starter-ui-ir/src/`
#   - `crates/starter-ui-bindings/src/`
#   - `packages/starter-sdui-react/src/`
#
# Excluded from scanning (also per SCOPE.md § R3 — "in source, not
# comments / tests / fixtures"):
#   - any path containing `/tests/` or `/fixtures/`
#   - any file ending in `.test.ts` / `.test.tsx`
#   - the `bin/` directory under `starter-ui-ir/src` (build-time
#     schema emitter, not a wire-format input)
#   - line and block comments (best-effort stripping)
#
# Usage:
#   scripts/check-sdui-domain-leak.sh          # check, exit 1 on drift
#   scripts/check-sdui-domain-leak.sh --update # rewrite words.txt files
#
# The `--update` mode is for the PR that legitimately adds new
# vocabulary. It is NOT a release valve for a domain leak — see R3.

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="check"
if [[ "${1:-}" == "--update" ]]; then
    MODE="update"
fi

# Returns the set of identifiers (lowercase, length ≥ 4) reachable
# from a given source root. Strips // ... and /* ... */ comments
# best-effort, then tokenises on the [A-Za-z_][A-Za-z0-9_]* grammar.
extract_words() {
    local src_dir="$1"
    if [[ ! -d "$src_dir" ]]; then
        echo "fatal: source dir missing: $src_dir" >&2
        exit 2
    fi
    # shellcheck disable=SC2016
    find "$src_dir" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' \) \
        ! -path '*/tests/*' \
        ! -path '*/fixtures/*' \
        ! -path "$src_dir/bin/*" \
        ! -name '*.test.ts' \
        ! -name '*.test.tsx' \
        -print0 \
    | xargs -0 awk '
        BEGIN { in_block = 0 }
        {
            line = $0
            # strip /* ... */ blocks (single-line)
            gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", line)
            # crude multi-line block tracking
            if (in_block) {
                if (match(line, /\*\//)) {
                    line = substr(line, RSTART + RLENGTH)
                    in_block = 0
                } else { next }
            }
            if (match(line, /\/\*/)) {
                line = substr(line, 1, RSTART - 1)
                in_block = 1
            }
            # strip // line comments (rust/ts) — naive but adequate
            sub(/\/\/.*$/, "", line)
            # strip leading # docstring-like lines? no — rust uses //
            print line
        }
    ' \
    | tr -c 'A-Za-z0-9_' '\n' \
    | awk 'length($0) >= 4' \
    | tr 'A-Z' 'a-z' \
    | sort -u
}

# Subtract the second sorted file from the first. Outputs lines that
# appear in `current` but not in the committed allowlist — i.e. the
# words the PR added without listing them.
diff_added() {
    comm -23 "$1" "$2"
}

CRATES=(
    "crates/starter-ui-ir/src|crates/starter-ui-ir/words.txt"
    "crates/starter-ui-bindings/src|crates/starter-ui-bindings/words.txt"
    "packages/starter-sdui-react/src|packages/starter-sdui-react/words.txt"
)

EXIT=0
for entry in "${CRATES[@]}"; do
    src_dir="${REPO_ROOT}/${entry%|*}"
    words_file="${REPO_ROOT}/${entry#*|}"
    tmp_current="$(mktemp)"
    extract_words "$src_dir" > "$tmp_current"

    if [[ "$MODE" == "update" ]]; then
        cp "$tmp_current" "$words_file"
        echo "updated: $(basename "$(dirname "$words_file")")/words.txt ($(wc -l < "$words_file") tokens)"
        rm -f "$tmp_current"
        continue
    fi

    if [[ ! -f "$words_file" ]]; then
        echo "fatal: allowlist missing: $words_file" >&2
        rm -f "$tmp_current"
        EXIT=2
        continue
    fi

    tmp_added="$(mktemp)"
    diff_added "$tmp_current" "$words_file" > "$tmp_added"
    if [[ -s "$tmp_added" ]]; then
        echo "R3 domain-leak gate FAILED for $(echo "$entry" | cut -d'|' -f1):" >&2
        echo "  the following identifiers/string-fragments appear in source" >&2
        echo "  but are NOT in $(echo "$entry" | cut -d'|' -f2):" >&2
        sed 's/^/    /' "$tmp_added" >&2
        echo "" >&2
        echo "  add each one to the allowlist (one per line, sorted) and" >&2
        echo "  justify the framework concept it represents in the PR" >&2
        echo "  description. 'convenience' is not a framework concept." >&2
        echo "  Or run: scripts/check-sdui-domain-leak.sh --update" >&2
        EXIT=1
    fi
    rm -f "$tmp_current" "$tmp_added"
done

if [[ "$MODE" == "check" && $EXIT -eq 0 ]]; then
    echo "R3 allowlist check passed for all three crates."
fi
exit $EXIT
