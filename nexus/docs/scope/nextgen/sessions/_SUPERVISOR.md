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
3. **Is the build red?** FIRST check if a wake is live (lock HELD + heartbeat PID alive). If so,
   the tree is mid-edit and a red `cargo check` is the EXPECTED in-progress state — a WS only
   compiles green when it commits at the end. **Do NOT diagnose or touch a red build while a wake
   is live.** Only evaluate the build when NO wake is running (lock FREE). Then: if a *committed*
   WS left HEAD not compiling, that poisons later WS. Safe repair only if it's an obvious mechanical
   break (missing `use`, unclosed brace) in HEAD's last commit AND in nextgen/loop-owned files →
   fix + commit `supervisor: fix build break in <file>`. If substantive, or uncommitted, or in
   another session's lane (`nexus-ai`, `agents/`) → do NOT touch; log to TODOs.md + supervisor log.
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
- 2026-06-09 13:13 — All healthy. Cron firing (20:00/05/10). WS-07 🔵, wake PID 136336 ALIVE, lock
  HELD. `cargo check` is RED (lettre dep not yet wired, NewRule/RulePatch new fields, agents arg
  count) — but ALL broken files are UNCOMMITTED WIP (email/slack/condition/policy/reduce/template.rs
  are ?? ; no WS-07 commit yet). This is the EXPECTED mid-edit state of a live wake, NOT a committed
  break. Correctly took NO action (touching it would corrupt in-flight work + violate lane). Refined
  check #3: don't evaluate the build while a wake is live. WS commits: 7.
- 2026-06-09 13:18 — All healthy. Cron firing (20:05/10/15). WS-07 🔵 still building (wake PID
  136336, ~13 min in — normal; WS-03/04 took 20-25). Verified PROGRESSING not hung: alerting/
  template.rs touched <3 min ago. Lock HELD (kernel-owned), no STOP. Build red = same live WIP as
  13:13 (uncommitted) — no action per refined check #3. Only open TODO is the ✅ RESOLVED WS-03 one.
  WS commits: 7→8 (8th is a supervisor log commit, not a WS). No worker repair needed.
- 2026-06-09 13:23 — Healthy, WATCHING two things. Cron firing (20:10/15/20). WS-07 🔵, wake PID
  136336 ALIVE ~18 min (within WS-04's 25-min envelope). No alerting src touched in 4 min + subagent
  0% CPU → likely in final verify/commit phase (not a stall yet; threshold is ~25 min / no-commit-3-
  firings). Other session committed 5075c389 (nexus-ai/agents/skills) — touched
  tests/routes/alerts/alert_e2e_test.rs (+1), a cross-lane nudge into WS-07's area; flagging as a
  possible commit-conflict risk for WS-07. NO action (WS-07 alive, lock kernel-held). NEXT WAKE:
  if WS-07 still 🔵 with no commit (~23+ min) → approaching stall, inspect harder. Real WS commits: 5.
- 2026-06-09 13:28 — WS-07 ✅ DONE & committed clean (f52d6b38, 3041 lines: multi-condition rules,
  no-data/error policy, Slack+Email/lettre, templating, retry; migration 1001; 36 alerting tests +
  pnpm 134 all GREEN). My earlier `git log -4` just didn't reach it — wider search confirmed the
  commit. Both 13:23 watch-items cleared: NOT a stall (it was committing), and the cross-lane
  alert_e2e_test.rs touch caused NO conflict (WS-07 committed cleanly over it). WS-07 even fixed a
  real UI↔backend operator-name bug, and correctly DEFERRED C6 audit/undo (WS-12 substrate not run)
  + left the other session's `pub mod agents;` hunk untouched. Wake PID 136336 (23min) in tail,
  about to spawn WS-10 (next pending). NO action. Real WS commits: 5→6 (WS-03,04,07 done). All healthy.
- 2026-06-09 13:32 — Clean autonomous handoff, NO action. Cron firing (20:20/25/30). WS-07 ✅ fully
  committed (f52d6b38 + hash row 84efc8ee). WS-10 (Kinds) now 🔵 — FRESH wake, new PID 167125
  (started 13:30); old 136336 exited cleanly after WS-07. Heartbeat rolled to new PID correctly.
  Lock HELD by live wake (kernel-owned), no STOP. WS-10 reuses the WS-03 binder — dependency order
  paying off. Only "blocker" is the already-✅RESOLVED WS-03 typecheck note. Real WS commits: 6 done
  (WS-03,04,07). 3/12 complete, WS-10 building. System self-driving.
- 2026-06-09 13:37 — All healthy, NO action. Cron firing (20:25/30/35). WS-10 (Kinds) 🔵, wake PID
  167125 ALIVE 7.5min, actively writing a clean verb-per-file kinds module (kind/load/lint/validate/
  resolve/manifest/error.rs all touched <4min) — the HOW-TO-CODE file-layout standard is landing.
  Lock kernel-held, no STOP. WS-10.md not created yet (normal at 7min; charter requires it — will
  confirm next wake). Only "blocker" is the ✅RESOLVED WS-03 note. Real WS commits: 6 done (03,04,07).
- 2026-06-09 13:42 — Healthy, NO action. Cron firing (20:30/35/40). WS-10 🔵 wake PID 167125 ALIVE
  12min, actively wiring kind dispatch (kinds/dispatch.rs + routes/query/run.rs touched <4min —
  matches ROADMAP §4 run.rs kind-dispatch). Lock kernel-held, no STOP. MINOR WATCH: WS-10.md session
  doc still not created at 12min (charter requires it; WS-04/07 effectively wrote theirs near commit
  time). Not blocking, not acting on a live wake. ESCALATE ONLY IF: WS-10 commits ✅ without ever
  producing WS-10.md — that's a real DoD miss to flag next wake. Real WS commits: 6 done (03,04,07).
- 2026-06-09 13:47 — Healthy, NO action. Cron firing (20:35/40/45). WS-10 🔵 wake PID 167125 ALIVE
  17min (within WS-07's 23-min envelope). PROGRESSING + in finishing phase: openapi.json + ui
  generated/index.ts touched <3min = DTO-first codegen step (regenerate OpenAPI → pnpm codegen),
  which only runs near commit; still refining kinds lint/validate. Lock kernel-held, no STOP.
  WS-10.md STILL missing at 17min — watch stands: escalate only if WS-10 commits ✅ without it
  (still building, so not yet). Real WS commits: 6 done (03,04,07).
