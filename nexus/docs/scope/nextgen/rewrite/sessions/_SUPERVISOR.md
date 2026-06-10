# Nexus Rewrite — Supervisor Loop (the watchdog)

> This is what the in-session `/loop` (the supervisor) does every wake. It runs ALONGSIDE the
> OS cron, which is the worker. **The cron spawns workstreams; the supervisor does NOT.**
> Two roles, one branch (`nexus-rewrite`), no collision — because the supervisor never takes the
> loop lock and never spawns an RW.
> (This doc inherits every lesson from the 2026-06-09 nextgen run's supervisor — see
> `../../sessions/_SUPERVISOR.md` for the original log, including the never-rm-the-lock incident.)

## Hard boundary (so the two loops never fight)
- The supervisor **never** acquires `sessions/.loop.lock` and **never** runs `loop-tick.sh` or
  spawns a workstream subagent. Spawning is the cron's exclusive job.
- The supervisor only **reads** state, **repairs** safe things, and **escalates** the rest.
- If the supervisor and cron would both write STATUS.md, the supervisor yields — it appends to the
  `## Supervisor log` below, never edits the queue rows the cron owns.

## Each wake, the supervisor checks (read-only first):
1. **Is the cron even firing?** Check `cron.log` for a firing in the last ~6 min and
   `crontab -l | grep nexus-rewrite-loop`. If the entry vanished, re-install it (safe repair).
   If cron is installed but silent → escalate loudly in the log.
2. **Is a wake stuck?** flock is the source of truth; a HELD lock ALWAYS means a live holder.
   NEVER `rm` the lock. Liveness: read PID from `.loop.heartbeat`, `kill -0 <pid>`. Alive → healthy
   wake in flight, leave it. The only legitimate protective tool is a PID-guarded `.loop.STOP`.
3. **Is the build red?** FIRST check whether a wake is live (lock HELD + heartbeat PID alive).
   A red `cargo check` during a live wake is the EXPECTED mid-edit state — do NOT touch it.
   Only evaluate the build when the lock is FREE. If a *committed* RW left HEAD broken: safe repair
   only for an obvious mechanical break (missing `use`, unclosed brace) in rewrite-lane files from
   HEAD's last commit → fix + commit `supervisor: fix build break in <file>`. Anything substantive
   or out-of-lane → TODOs.md + supervisor log, hands off.
4. **Is the queue progressing?** No new RW commits for ~30 min AND no row legitimately 🔵-building
   → investigate (blocked row? cron dead? all done?) and escalate.
5. **Fresh ⛔ rows / new TODOs?** Surface them in the supervisor log. The supervisor does NOT
   resolve design blockers (human's call) — it makes them visible and confirms the loop moved on.
6. **All ✅ or only-blocked-remaining?** The run is done → say so, confirm the cron was removed
   (or remove it), write the final human report, and stop the supervisor loop.

## What the supervisor may safely repair (no human needed)
- Re-install a vanished cron entry.
- Fix a trivial mechanical compile break in rewrite-lane files committed in the last RW.
- Mark a row ⛔→⬜ when its TODOs.md blocker has a `✅ RESOLVED` line (so cron re-runs it).
- Remove a PID-guarded `.loop.STOP` once its guarded PID has exited. (A bare STOP is a human kill
  switch — NEVER auto-remove.)

## What the supervisor must NOT do
- Spawn a workstream, take the loop lock, run loop-tick.sh, or `rm` the lock file.
- Edit another session's files — different lane.
- Resolve a design-ambiguity blocker by guessing — the no-questions rule routes those to the human.
- Revert or force-push anything.

## Supervisor log
<!-- Appended each wake: `YYYY-MM-DD HH:MM — <what was checked / repaired / escalated>` -->
- 2026-06-10 02:19 — Supervisor online (first wake, pre-first-firing). Cron entry INSTALLED
  (nexus-rewrite-loop, */5); no cron.log/heartbeat/lock yet — correct, first firing due 02:20Z.
  Branch nexus-rewrite created at a4a4b63a (scope docs committed). Queue: 8 rows, all ⬜, none 🔵,
  no STOP, no TODOs. NO action needed. NEXT WAKE: confirm the 02:20Z firing happened (cron.log
  exists, wake-start/complete in heartbeat) and RW-01 went 🔵.
- 2026-06-10 02:24 — All healthy, NO action. Cron fired on schedule (02:20:01Z, first firing).
  RW-01 🔵 — wake PID 716380 ALIVE ~4min, lock correctly HELD (kernel-owned, untouched), no STOP.
  ACTIVELY BUILDING the exact RW-01 spec layout: core/{node,pipeline,registry,config,error,
  outcome}.rs all touched <5min + the sanctioned lib.rs append; RW-01.md session doc already
  created (charter followed early — good sign). Build NOT evaluated (lock HELD = mid-edit, per
  check #3). No TODOs, queue progressing (0✅/1🔵/7⬜). NEXT WAKE: expect RW-01 nearing tests or
  commit; gate fires on the cron side when its row reads Done.
