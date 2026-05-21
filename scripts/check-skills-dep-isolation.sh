#!/usr/bin/env bash
# check-skills-dep-isolation.sh — CI gate for starter-skills's
# transitive normal-deps closure (agent R2: provider SDK isolation;
# DOCS/agent/SKILLS.md per-job SCOPE Stage 12 dep-tree gate).
#
# `LlmSkillSelector` MUST talk to providers only through the
# `AiRunner` trait (which lives in `starter-spi`). The selector
# itself must not pull a provider SDK into starter-skills's normal
# deps — that would let a single Cargo.toml edit drag every provider
# crate into every consumer of starter-skills.
#
# This script fails fast on any of the banned crate names in
# `cargo tree -p starter-skills --edges normal`. The list is the
# exact one named by the per-job SCOPE Stage 12 brief; additions go
# via PR (decision-change protocol in DOCS/agent/SKILLS.md).
#
# Usage:
#   scripts/check-skills-dep-isolation.sh
#
# Exit codes:
#   0  no banned provider SDK in starter-skills's normal-deps closure
#   1  one or more banned crates found (offending lines printed)
#   2  invocation / environment error

set -euo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "fatal: cargo not on PATH" >&2
    exit 2
fi

# Banned crate names. These are the provider-SDK crates the per-job
# SCOPE Stage 12 brief enumerates by name. Match on the start of the
# line so e.g. `async-openai` does not also match a hypothetical
# `something-async-openai-wrapper` re-export.
BANNED=(
    "async-openai"
    "anthropic-ai-sdk"
    "anthropic-sdk"
    "google-genai"
    "aws-sdk-bedrockruntime"
    "mistralai"
    "ollama-rs"
)

TREE="$(mktemp)"
trap 'rm -f "${TREE}"' EXIT

# `--edges normal` strips dev-dependencies — the selector tests in
# starter-skills are free to use a hand-written fake AiRunner, but
# the normal-dep closure (what downstream consumers actually link)
# must stay provider-SDK-free.
cargo tree --manifest-path "${REPO_ROOT}/Cargo.toml" \
    -p starter-skills --edges normal --prefix none 2>/dev/null \
    | sed -e 's/ (\*)//' \
          -e "s| (${REPO_ROOT}/[^)]*)||" \
    > "${TREE}"

FOUND=0
for crate in "${BANNED[@]}"; do
    # Match the crate name as a whole token followed by a space + a
    # version (cargo tree's normalised output). Anchored to start of
    # line + word boundary to avoid substring false positives.
    if grep -E "^${crate} v[0-9]" "${TREE}" >/dev/null 2>&1; then
        echo "starter-skills: banned provider SDK found in normal-deps closure:"
        grep -E "^${crate} v[0-9]" "${TREE}"
        FOUND=$((FOUND + 1))
    fi
done

if [[ "${FOUND}" -gt 0 ]]; then
    echo
    echo "starter-skills's normal-deps closure must stay provider-SDK-free"
    echo "(agent R2 + DOCS/agent/SKILLS.md per-job SCOPE Stage 12)."
    echo "LlmSkillSelector talks to providers only through AiRunner —"
    echo "remove the offending dep from starter-skills's transitive"
    echo "closure, do not relax this gate."
    exit 1
fi

echo "starter-skills dep isolation: no banned provider SDK in normal-deps closure."
exit 0
