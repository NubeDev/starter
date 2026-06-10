#!/usr/bin/env bash
# One wake of the Nexus REWRITE build loop. Fired by cron every 5 minutes (see install-cron.sh).
# Acquires an exclusive lock so overlapping firings can't double-spawn a workstream, honors the
# STOP sentinel kill switch, then runs ONE pass of the LOOP ALGORITHM in _ORCHESTRATION.md headless.
set -euo pipefail

# Cron runs with a stripped PATH that lacks claude/node/pnpm/cargo — without this the tick dies
# with "claude: command not found" and the whole loop silently no-ops.
export PATH="/home/user/snap/code/226/.local/share/pnpm:/home/user/.nvm/versions/node/v22.22.0/bin:/home/user/.cargo/bin:/home/user/.local/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/go/bin:/home/user/go/bin:$PATH"
CLAUDE_BIN="/home/user/snap/code/226/.local/share/pnpm/claude"

REPO="/home/user/code/rust/starter"
SESS="$REPO/nexus/docs/scope/nextgen/rewrite/sessions"
LOG="$SESS/cron.log"
LOCK="$SESS/.loop.lock"
STOP="$SESS/.loop.STOP"

ts() { date -u +%FT%TZ; }

# Kill switch: `touch .loop.STOP` to halt the run without editing crontab.
if [[ -f "$STOP" ]]; then
  echo "$(ts) STOP sentinel present — exiting without spawning." >>"$LOG"
  exit 0
fi

# Single-firing lock. -n = fail immediately if another firing holds it. flock is the SOLE mutex;
# the kernel releases it when the holder dies (even SIGKILL) — never `rm` the lock to "recover".
exec 9>"$LOCK"
if ! flock -n 9; then
  echo "$(ts) another firing holds the lock — skip." >>"$LOG"
  exit 0
fi

cd "$REPO"
echo "$(ts) firing one wake." >>"$LOG"

# Heartbeat records THIS firing's PID so a watcher can tell alive-vs-dead with `kill -0 <pid>`.
HEARTBEAT="$SESS/.loop.heartbeat"
echo "$(ts) wake-start pid=$$" >"$HEARTBEAT"

# One headless wake. Claude reads the driver doc, runs the LOOP ALGORITHM once, spawns/gates one
# RW, updates STATUS, and exits. --dangerously-skip-permissions because cron is non-interactive;
# the work is confined to this repo on branch nexus-rewrite.
"$CLAUDE_BIN" -p "Read nexus/docs/scope/nextgen/rewrite/sessions/_ORCHESTRATION.md and execute exactly ONE wake of the LOOP ALGORITHM (headless cron mode section applies), then exit. You are on branch nexus-rewrite. Do not ask questions; a blocked workstream logs to rewrite/sessions/TODOs.md and the next pending one is chosen. Spawn the workstream session as a subagent using the AGENT CHARTER verbatim. Commit only files the workstream owns. When you have spawned or gated exactly one workstream, append a one-line entry to rewrite/sessions/STATUS.md's loop log and stop." \
  --model claude-opus-4-8 \
  --dangerously-skip-permissions \
  >>"$LOG" 2>&1

echo "$(ts) wake-complete pid=$$" >"$HEARTBEAT"
echo "$(ts) wake complete." >>"$LOG"
