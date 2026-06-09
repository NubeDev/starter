# Nexus Build — Supervisor Loop (the watchdog)

> This is what the in-session `/loop` (the supervisor) does every wake. It runs ALONGSIDE the
> OS cron, which is the worker. **The cron spawns workstreams; the supervisor does NOT.**
> Two roles, one branch (`nexus-gaps`), no collision — because the supervisor never takes the
> loop lock and never spawns a WS.

## Hard boundary (so the two loops never fight)
- The supervisor **never** acquires `sessions/.loop.lock` and **never** runs `loop-tick.sh` or
  spawns a workstream subagent. Spawning is the cron's exclusive job.
- The supervisor only **reads** state, **repairs** safe things, and **escalates** the rest.
- If the supervisor and cron would both write STATUS.md, the supervisor yields — it appends to a
  separate `## Supervisor log` section, never edits the queue rows the cron owns.

## Each wake, the supervisor checks (read-only first):
1. **Is the cron even firing?** `grep nexus-build-loop /var/log/syslog | tail -3` — confirm a
   firing in the last ~6 min. If cron has gone silent, that's the #1 failure → escalate loudly
   (and check `crontab -l` still has the entry; if the entry vanished, re-install it — that's a
   safe repair).
2. **Is a wake stuck?** Lock HELD + `.loop.heartbeat` shows `wake-start` older than ~25 min →
   the worker died mid-wake. Safe repair: clear the stale lock so the next cron firing can
   recover the WS (the WS work is idempotent; it resumes from committed state).
3. **Is the build red?** Run `cargo check` (fast) on the workspace. If a committed WS left the
   tree not compiling, that poisons every later WS. Safe repair: if it's an obvious, in-the-last-
   commit, mechanical break (missing `use`, unclosed brace) AND it's in nextgen/loop-owned files,
   fix it and commit `supervisor: fix build break in <file>`. If it's substantive or in another
   session's lane (e.g. `nexus-ai`), do NOT touch — log it to TODOs.md and note in supervisor log.
4. **Is the queue progressing?** Compare `git log` WS-commit count vs the previous wake. If no new
   WS commits for ~30 min AND no row is legitimately 🔵-building, the queue has stalled →
   investigate (blocked row? cron dead? all done?) and escalate.
5. **Are there fresh ⛔ blocked rows / new TODOs?** Surface them in the supervisor log so the
   human sees them without digging. The supervisor does NOT resolve design blockers (those are the
   human's call) — it only makes them visible and confirms the loop moved on to the next WS.
6. **All ✅ or only-blocked-remaining?** The run is done → say so, and note the loop can be stopped.

## What the supervisor may safely repair (no human needed)
- Re-install a vanished cron entry.
- Clear a stale lock from a provably-dead wake (heartbeat > 25 min + no claude process).
- Fix a trivial, mechanical compile break in loop/nextgen-owned files committed in the last WS.
- Mark a row ⛔→⬜ when its TODOs.md blocker has a `✅ RESOLVED` line (so cron re-runs it).

## What the supervisor must NOT do
- Spawn a workstream, take the loop lock, or run loop-tick.sh.
- Edit another session's files (e.g. `nexus-ai/**`) — different lane.
- Resolve a design-ambiguity blocker by guessing — that's the human's, per the no-questions rule.
- Revert or force-push anything.

## Supervisor log
<!-- Appended each wake: `YYYY-MM-DD HH:MM — <what was checked / repaired / escalated>` -->
