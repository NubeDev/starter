#!/usr/bin/env bash
# CI dep-tree gate per Insights SCOPE R-ins-5 (and agent R2 by
# analogy with `starter-flow-node-loop`).
#
# `starter-insights` must NOT pull any AI provider SDK into its
# dependency tree. The AiRunner trait lives in `starter-spi`; the
# concrete provider SDKs (`anthropic-ai-sdk`, `async-openai`,
# `claude-wrapper`, ...) live in `starter-ai`. A consumer that wants
# AI rule kinds (`rule.ai-check` / `rule.ai-debug`) wires an
# `AiJudge` / `AiDebugger` impl at boot — `starter-insights` itself
# stays SDK-free so the headless-appliance / dep-audit story holds.
#
# Fails the CI step if any of the forbidden crates appears in
# `cargo tree -p starter-insights` (default features + sqlite).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

FORBIDDEN=(
    "anthropic-ai-sdk"
    "async-openai"
    "claude-wrapper"
    "openai-rs"
)

# Use `--no-default-features` then `--all-features` to cover both
# the headless build and the maximal build.
for feature_set in "--no-default-features" "--features sqlite"; do
    tree=$(cargo tree -p starter-insights $feature_set --edges normal --prefix none 2>/dev/null \
        | awk '{print $1}' | sort -u)
    for crate in "${FORBIDDEN[@]}"; do
        if echo "$tree" | grep -qx "$crate"; then
            echo "R-ins-5 violation: starter-insights pulls $crate ($feature_set)" >&2
            echo "Forbidden — provider SDKs must live behind AiJudge / AiDebugger impls wired in the host" >&2
            exit 1
        fi
    done
done

echo "starter-insights dep tree: OK (no provider SDKs)" >&2
