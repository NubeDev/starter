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
- 2026-06-09 13:51 — Healthy, NO action. Cron firing (20:40/45/50). WS-10 🔵 wake PID 167125 ALIVE
  22min (≈ WS-07's 23; large WS: kinds engine + dispatch + codegen + UI picker). NOT wedged —
  6 files touched <2min, now UI-side (KindPicker.tsx/Explore.tsx/api/query/kinds.ts), live
  `pnpm --filter`+esbuild running = final frontend build/codegen phase. kinds/ backend module
  present uncommitted (?? ). Lock kernel-held, no STOP. WS-10.md still missing — watch unchanged
  (escalate only if it commits ✅ without the doc). Real WS commits: 6 done (03,04,07).
- 2026-06-09 13:56 — WS-10 ✅ DONE & committed clean (dd4687ae + hash row 7a442053). BUILD GREEN —
  first lock-FREE wake, `cargo check --workspace` finished clean in 21s with WS-10 at HEAD (real
  compile confirmation, not a mid-edit snapshot). Declarative query-kinds: registry/loader/lints/
  core pack (meters_list, meter_get, usage_bucketed, top_sites_by_usage) + kind-mode dispatch +
  GET /query/kinds + KindPicker UI, reusing the WS-03 binder (no 2nd engine). 19 kinds unit tests +
  pnpm green. FALSE-ALARM CLOSED: the "WS-10.md missing" watch (carried 4 wakes) was a GREP artifact
  — the doc EXISTS (7009 bytes, committed in 7a442053) but uses `**Status:**` (bold) so my
  `^Status:` grep missed it. NO real DoD miss. FIX: future wakes grep `[Ss]tatus.*Done` format-
  agnostic. Wake exited clean, lock FREE, no STOP. 4/12 done (03,04,07,10). Next: WS-08.
- 2026-06-09 14:01 — All healthy, NO action. Cron firing (20:50/55, 21:00). Clean handoff: WS-10 ✅,
  WS-08 (Connector breadth) now 🔵 — fresh wake PID 187071 (started 14:00), 1.3min in. Heartbeat
  rolled to new PID, lock kernel-held, no STOP. WS-08.md not yet (normal at 1min; format-agnostic
  check confirms row present). NOTE: commit 52f5e72c "supervisor: 14:00 wake" was written by the
  CRON wake itself (it narrates its spawn action) — NOT a second supervisor; cron + I both write
  nextgen docs in our own sections, no conflict. Only blocker is the ✅RESOLVED WS-03 note.
  4/12 done (03,04,07,10); WS-08 building.
- 2026-06-09 14:06 — All healthy, NO action. Cron firing (20:55, 21:00/05). WS-08 (Connectors) 🔵
  wake PID 187071 ALIVE 6min, actively writing datasource/postgres/probe.rs + mod.rs (<4min) —
  on-scope connector probing. Lock kernel-held, no STOP. WS-08.md not yet at 6min (normal; sessions
  write it near commit — NOT re-opening a multi-wake watch, just confirm with commit). Only blocker
  is the ✅RESOLVED WS-03 note. 4/12 done (03,04,07,10); WS-08 building.
- 2026-06-09 14:10 — All healthy, NO action. Cron firing (21:00/05/10). WS-08 🔵 wake PID 187071
  ALIVE 11min, PROGRESSING into later phase: openapi.json + UI datasource forms
  (DatasourceFormDialog.tsx, test-connection.ts, useDatasourceMutations.ts) touched <3min = DTO
  codegen + per-kind connector UI (WS-08-owned per §4). Lock kernel-held, no STOP. Only blocker is
  ✅RESOLVED WS-03 note. 4/12 done (03,04,07,10); WS-08 in mid/late build.
- 2026-06-09 14:15 — FIRST REAL BLOCKER (escalating to human). Cron firing (21:05/10/15). WS-08 ⛔
  blocked — LEGITIMATE "ask-don't-guess" call: MQTT/Modbus need new gated deps (rumqttc/tokio-modbus,
  a §9 human dep-policy decision) AND the datasource-kind config format = WS-10 datasource-kinds
  (Wave 2) which is NOT YET BUILT (WS-10 shipped only query-kinds). WS-08 correctly refused to invent
  the format or add heavy deps; recommends running WS-10 datasource-kinds first, then re-run WS-08.
  It DID fully land the pre-save POST /datasources/test acceptance criterion (test_connection.rs,
  probe.rs, UI button, mirrored tests). STATE TO WATCH NEXT WAKE: wake PID 187071/child 187082 still
  ALIVE 16min, partial work still UNCOMMITTED (test_connection.rs/probe.rs ?? , form dialogs M).
  Heartbeat=wake-start (not complete) → it logged blocker + marked ⛔ but has NOT yet committed the
  partial or exited. NOT forcing (under 25-min/3-firing stall threshold; flock self-heals). If NEXT
  wake shows 187071 still alive >25min OR exited with the partial still uncommitted → that's a real
  problem to handle. NO supervisor action taken (can't commit WS-08's lane, can't rm the lock).
  HUMAN DECISION NEEDED: approve "run WS-10 datasource-kinds first + gate rumqttc/tokio-modbus off
  by default", or pick another path. 4/12 done; WS-08 partial+blocked.
- 2026-06-09 17:48 — HUMAN DECISION recorded + queue extended. While escalating WS-08, the loop had
  already raced ahead: 10/12 ✅ (WS-01/02/05/06/11/12 all done since the blocker), WS-09 (last) now
  🔵 (PID 330356), WS-08 correctly skipped+honored. Human (ap, away from PC, "no questions") decided
  WS-08 connectors: run WS-10 datasource-kind FORMAT first then MQTT against it; **Modbus DROPPED**;
  rumqttc gated OFF by default. Actioned: added STATUS queue row 13 "WS-08b" (⬜) so the loop picks
  it up after WS-09; wrote the decision + relaxed acceptance into the WS-08 TODO. Did NOT touch
  WS-09's lane (committed only my docs). Blockers list = healthy: mostly "deferred follow-up NOT a
  blocker" + 2 pre-existing out-of-lane. NO further questions per human instruction — pure autonomous
  from here. 10/12 done, WS-09 building, WS-08b queued.
- 2026-06-10 00:50 — All healthy, NO action, NO REAL BLOCKERS remaining. Cron firing (00:40/45/50).
  WS-09 (last core WS) 🔵 wake PID 330356/child 330360 ALIVE ~15min, actively writing its scoped
  modules: cache/key.rs + cache/store.rs (C3 cache-key tuple — all inputs WS-01/02/11 now committed,
  ordering paid off), ratelimit/bucket.rs, quota/limiter.rs + main/serve/state wiring (§4 shared
  mounts). Lock kernel-held, no STOP. WS-08b queued ⬜ (row 13) per human decision — will run after
  WS-09. Filtered blocker list EMPTY (all TODOs now resolved/human-decided/deferred-follow-up/pre-
  existing-out-of-lane). 10/12 core done; WS-09 building → then WS-08b. Autonomous per human "no
  questions" instruction.
- 2026-06-10 00:54 — MILESTONE: WS-09 ✅ committed clean (813b01c4 — C3-keyed TTL query cache w/
  single-flight + units_locale_tz placeholder per D4, per-tenant quota semaphore, token-bucket
  rate-limit layer, run_cached seam; 94 lib tests green). **ALL 12 ORIGINAL CORE WORKSTREAMS NOW
  RESOLVED: 11 ✅ DONE (03,04,07,10,01,02,11,12,05,06,09) + WS-08 deferred→WS-08b per human.** Only
  WS-08b (⬜, human-decided MQTT-only) remains in queue. Loop log shows WS-09 wake logged complete,
  but wake PID 330356/330360 still ALIVE 20min + lock HELD + WS-08b still ⬜ + nothing touched 3min:
  WS-09 deliverable IS committed (no queue risk) — this is process-exit latency / pre-WS-08b-spawn
  gap, NOT a wedge (under 25-min/3-firing threshold; flock self-heals). NEXT WAKE: expect WS-08b
  spawned; if WS-08b still ⬜ + this wake exited without spawning → next cron firing picks it up;
  only escalate if WS-08b stays ⬜ across 3+ firings with lock FREE. No real blockers. NO action.
- 2026-06-10 00:59 — All healthy, NO action. Cron firing (00:45/50/55). **FULL-STACK BUILD GREEN:**
  first lock-FREE check since WS-09 — `cargo check --workspace` clean in 22s against committed HEAD
  with ALL 11 done workstreams integrated (real cross-WS integration proof, not per-WS). WS-09 wake
  exited CLEAN (heartbeat wake-complete, PID 330356 gone, lock FREE) — confirms last wake's "20min
  still alive" was just exit latency, not a wedge. WS-08b still ⬜: the WS-09 firing gated+exited
  without spawning next (normal one-action-per-firing); NEXT cron firing (~01:00) picks up WS-08b as
  first pending. Not a stall (lock free, <3 firings). No STOP, no real blockers. 11/12 ✅ + WS-08b
  queued. WATCH: confirm WS-08b spawns next wake.
- 2026-06-10 01:04 — All healthy, NO action. Cron firing (00:50/55, 01:00). FINAL item WS-08b 🔵 —
  spawned 18:00 (fresh wake PID 344587/child 344599), 4min in, ACTIVELY building datasource_kinds/
  manifest.rs + error.rs (the WS-10 datasource-kind FORMAT first, exactly per the human decision).
  WS-08b.md created (3117b) and explicitly records the human constraints: "build the format FIRST,
  then MQTT; Modbus DROPPED; rumqttc gated OFF" — confirms the autonomous-with-human-decision flow
  works end-to-end (session read the decision from TODOs/STATUS). Lock kernel-held, no STOP, no real
  blockers. 11/12 ✅ + WS-08b building = the LAST item. After this commits, the entire queue is done.
- 2026-06-10 01:09 — All healthy, NO action. Cron firing (00:55, 01:00/05). WS-08b 🔵 (final item)
  wake PID 344587/child 344599 ALIVE 9min, PROGRESSED format→MQTT exactly per human decision:
  datasource_kinds/validate.rs (format) + datasource/mqtt/probe.rs + mqtt/mod.rs + datasource-kinds/
  mqtt_config.json (MQTT kind manifest) + Cargo.toml (gated rumqttc) + main/state/openapi wiring.
  NO Modbus files anywhere — correctly dropped. Lock kernel-held, no STOP, no real blockers. Only
  incomplete row is WS-08b 🔵 (WS-08 ⛔ is the superseded original). 11/12 ✅; WS-08b in mid/late
  build. After it commits → ENTIRE QUEUE DONE.
