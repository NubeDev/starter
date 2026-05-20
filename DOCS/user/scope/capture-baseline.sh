#!/usr/bin/env bash
#
# F-0.2: reproducible `cargo tree` baseline capture portable across
# `.codeless/worktrees/job-*` checkouts and CI nodes.
#
# The raw output of `cargo tree` embeds an absolute path to the crate
# manifest on the first line, e.g.
#
#     starter-spi v0.1.0 (/home/user/code/rust/starter/crates/starter-spi)
#
# Codeless runs jobs from per-worktree paths like
# `/home/user/.codeless/worktrees/job-01KS.../crates/starter-spi`, so a
# baseline captured in a worktree never matches one captured in the
# main repo. This script strips the absolute prefix down to the
# repo-relative tail so the baseline is byte-stable across worktrees.
#
# The strip rule preserves the path tail starting at `/crates/`,
# `/examples/`, or `/starter-extensions/crates/` — uniquely identifying
# the crate. Non-workspace crates print without a path and pass
# through untouched.
#
# Usage (from anywhere — the script re-anchors to the repo root):
#
#     DOCS/user/scope/capture-baseline.sh starter-spi \
#         > DOCS/user/scope/starter-spi-deps.baseline.txt
#
# Diff a candidate against the baseline:
#
#     DOCS/user/scope/capture-baseline.sh starter-spi \
#         | diff - DOCS/user/scope/starter-spi-deps.baseline.txt
#
# The script intentionally captures the default-feature view (no
# `--features` / `--no-default-features` flags); F-0.1 puts `units`,
# `i18n`, and `preferences` behind opt-in features so the default view
# is the contract every downstream crate inherits when it writes
# `starter-spi = { workspace = true }`.
#
# F-0.2 (DOCS/user/scope/TODO.md) closes once both
# starter-spi-deps.baseline.txt and
# DOCS/flow/scope/starter-flow-spi-deps.baseline.txt have been
# re-captured through this script.

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <crate>" >&2
    exit 2
fi

crate="$1"

# Re-anchor to the repo root so the script is callable from anywhere.
repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

cargo tree -p "$crate" --edges normal \
    | sed -E 's#\(/[^)]*/(starter-extensions/crates|crates|examples)/#(/\1/#g'
