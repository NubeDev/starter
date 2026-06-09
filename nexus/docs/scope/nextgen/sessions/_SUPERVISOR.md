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
2. **Is a wake stuck?** `flock` is the source of truth and the kernel frees it the instant the
   holder dies — so a HELD lock ALWAYS means a live holder. NEVER `rm` the lock to "recover"
   (that orphans a running wake and invites a double-spawn — a mistake made on 2026-06-09).
   To check liveness: read the PID from `.loop.heartbeat` (`wake-start pid=<N>`) and run
   `kill -0 <N>`. If the PID is alive → healthy wake in flight, leave it. If the lock reads HELD
   but no `loop-tick`/`claude -p` process exists, the kernel will already have freed it by the next
   firing — do nothing. The only legitimate repair is the STOP sentinel (see below), never `rm`.
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
- Fix a trivial, mechanical compile break in loop/nextgen-owned files committed in the last WS.
- Mark a row ⛔→⬜ when its TODOs.md blocker has a `✅ RESOLVED` line (so cron re-runs it).
- (Locks self-heal via the kernel — there is NO "clear the lock" repair. See check #2.)

## What the supervisor must NOT do
- Spawn a workstream, take the loop lock, or run loop-tick.sh.
- Edit another session's files (e.g. `nexus-ai/**`) — different lane.
- Resolve a design-ambiguity blocker by guessing — that's the human's, per the no-questions rule.
- Revert or force-push anything.

## STOP-sentinel auto-clear (specific recovery)
If `.loop.STOP` exists AND its content mentions a guarded PID (e.g. "protect in-flight WS-04 wake
(PID N)"), check `kill -0 N`. Once that PID has exited, the wake it guarded is done — remove
`.loop.STOP` so cron resumes. This lets a protective STOP clear itself without anyone watching.
(A STOP with NO guarded-PID note is a deliberate human kill switch — NEVER auto-remove that one.)

## Supervisor log
<!-- Appended each wake: `YYYY-MM-DD HH:MM — <what was checked / repaired / escalated>` -->
- 2026-06-09 13:00 — Cron healthy (firing 19:45/50/55). WS-03 ✅, WS-04 🔵 building (live cron wake
  PID 103333). MISTAKE: I `rm`'d the lock thinking it stale; it wasn't — 103333 held it legitimately.
  Recovered by placing a PID-guarded `.loop.STOP` so the 20:00 firing can't double-spawn WS-04.
  Hardened loop-tick.sh (heartbeat records pid; flock is sole mutex, never `rm`) + fixed check #2.
  Supervisor will auto-clear STOP once PID 103333 exits. WS commits: 3.
- 2026-06-09 13:04 — All healthy. Cron firing (19:50/55, 20:00). WS-04 ✅ committed (d27f293f —
  full panel editor, UI-only, no migration); WS-03 ✅. Guarded PID 103333 exited → STOP
  auto-cleared, cron free to spawn WS-07 next firing. Lock FREE, no stale state. Only open TODO is
  the WS-03 one already marked ✅ RESOLVED above it (no action). WS commits: 3→5. Queue advancing.
- 2026-06-09 13:08 — All healthy, fully autonomous cycle (no supervisor repair needed). Cron firing
  (19:55/20:00/20:05). WS-07 (Alerting) now 🔵 — live cron wake PID 136336 ALIVE, lock correctly
  HELD (kernel-owned, not touched). No STOP. Only open TODO is the WS-03 one already ✅ RESOLVED
  above it. WS commits: 5→6. The two-loop design is self-driving: cron spawned WS-07 on its own.
