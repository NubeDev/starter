#!/usr/bin/env bash
# Capture a `cargo tree` dep baseline that is portable across
# `.codeless/worktrees/job-*` checkouts and CI nodes.
#
# Usage:
#   DOCS/user/scope/capture-baseline.sh <crate> > <baseline-file>
#
# Example (re-capture starter-spi baseline against the default-feature
# build per D-0.1 / D-0.2):
#   DOCS/user/scope/capture-baseline.sh starter-spi \
#     > DOCS/user/scope/starter-spi-deps.baseline.txt
#
# The script:
#   1. runs `cargo tree -p <crate> --edges normal` from the repo root,
#   2. strips the worktree path that cargo embeds on workspace-member
#      lines (e.g. `(/home/user/.codeless/worktrees/job-XXXX/crates/...)`)
#      so the output diffs byte-for-byte across worktrees.
#
# The strip rule preserves the path's tail starting at `/crates/` or
# `/examples/` — that's the part that uniquely identifies the crate and
# is stable across worktrees. Non-workspace crates print without a
# path and are passed through untouched.
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
# Cargo.toml in the workspace root is the anchor.
repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

cargo tree -p "$crate" --edges normal \
  | sed -E 's#\(/[^)]*/(crates|examples)/#(/\1/#g'
