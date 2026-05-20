#!/usr/bin/env bash
#
# F-0.2: reproducible baseline-capture for `cargo tree -p starter-spi`.
#
# The raw output of `cargo tree` embeds an absolute path to the crate
# manifest on the first line, e.g.
#
#     starter-spi v0.1.0 (/home/user/code/rust/starter/crates/starter-spi)
#
# Codeless runs jobs from per-worktree paths like
# `/home/user/.codeless/worktrees/job-01KS.../crates/starter-spi`, so a
# baseline captured in the worktree never matches one captured in the
# repo. This script strips the absolute path to its repo-relative form
# so the baseline is byte-stable across worktrees.
#
# Usage (from repo root):
#
#     bash DOCS/user/scope/capture-baseline.sh starter-spi \
#         > DOCS/user/scope/starter-spi-deps.baseline.txt
#
# Diff a candidate against the baseline:
#
#     bash DOCS/user/scope/capture-baseline.sh starter-spi \
#         | diff - DOCS/user/scope/starter-spi-deps.baseline.txt
#
# The script intentionally captures the default-feature view (no
# `--features` / `--no-default-features` flags); F-0.1 puts `units`,
# `i18n`, and `preferences` behind opt-in features so the default view
# is the contract every downstream crate inherits when it writes
# `starter-spi = { workspace = true }`.

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <crate-name>" >&2
    exit 2
fi

crate="$1"

cargo tree -p "${crate}" --edges normal \
    | sed -E 's#\(/[^)]*/(starter-extensions/crates|crates)/([a-zA-Z0-9_-]+)\)#(\1/\2)#g'
