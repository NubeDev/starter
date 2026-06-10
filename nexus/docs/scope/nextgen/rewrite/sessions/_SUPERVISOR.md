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
- 2026-06-10 03:05 — PEER REVIEW APPLIED (human-directed doc updates, my doc lane only).
  RW-01 ✅ committed+gated fast (9757df7c + 84ae2962, 5 core tests green, 02:58). Landed review
  edits as c046ebaf: §6 gains &mut-self processors, max_batch_rows slicing, schema-stability,
  source_on_error policy, post-RW-03 dep-bump note; §8 de-approves Polars (RW-06 now
  DataFusion-first spike, Arrow-C-FFI condition if Polars returns); RW-04 → COPY BINARY +
  Parquet size-rotation; RW-05 → evaluate datafusion-table-providers + MemoryPool input bound;
  RW-08 → fat-batch soak case + dlq option. DELTA: RW-01 shipped pre-review contract
  (&self, no batch bound) — logged in TODOs.md as a non-gate-failing follow-up AND pinned as
  step 0 in RW-02's spec. RW-02 went 🔵 (spawned 02:30 per loop log; may have read the
  pre-review spec — step 0 + TODOs cover reconciliation either way; if its commit lacks the
  alignment, next supervisor wake escalates it as an RW-03-blocking note, since traits freeze
  at RW-02 gate). Cron healthy, lock state not relevant to this doc-only action. Left
  uncommitted: STATUS.md RW-02 row (cron's lane) + out-of-lane diffs (nexus-store bind/scan,
  extensions/boot.rs, WS-14 doc — concurrent session, untouched).
- 2026-06-10 02:32 — All healthy, NO action. Cron firing on schedule (02:25 wake gated RW-01 ✅
  9757df7c, full log line present; 02:30:01 firing spawned RW-02). RW-02 🔵 — wake PID 737438
  ALIVE ~2.5min, lock correctly HELD, no STOP; no engine files touched yet (doc-reading phase,
  normal). Build NOT evaluated (lock HELD, check #3). Queue 1✅/1🔵/6⬜, TODOs: 1 real entry
  (the RW-01 contract-delta follow-up — non-blocking by design). WATCH (tight race): RW-02
  spawned 02:30:01; the step-0 alignment pin (d7850167) + TODOs entry (c046ebaf) committed
  ~02:31 — the subagent may have read the PRE-review spec. At RW-02's gate, verify
  core/node.rs has `process(&mut self, …)` + max_batch_rows slicing landed; if missing, that
  is an RW-03-BLOCKING fix pass (same-charter, gate step 4), because §6 traits freeze at the
  RW-02 gate. node.rs:33 currently still `&self` (expected — RW-02 just started). Cosmetic:
  RW-01's row says Finished 02:58 UTC, which is in the future vs real 02:32Z — subagent wrote
  a bad timestamp; harmless, not touching another session's log line.
- 2026-06-10 02:37 — All healthy, NO action. Cron firing correctly (02:35 firing skipped on
  held lock — exactly the designed behavior). RW-02 🔵 wake PID 737438 ALIVE ~7min, ACTIVELY
  building its exact lane: processor/sql.rs + processor/json_to_arrow.rs + arrow_json.rs +
  source/interval.rs all touched <4min (DataFusion-direct processors = the spec's step 1-2).
  Lock kernel-HELD, no STOP, build not evaluated (check #3). Queue 1✅/1🔵/6⬜. WATCH stands:
  node.rs:33 still `&self` — fine mid-build; judge ONLY at RW-02's gate (step-0 alignment may
  land late in its pass). NOTED out-of-lane: new commit 6e75fc5d "got extesions working"
  (extensions lane — concurrent session/human); not touched per lane rules; the RW-02 gate's
  full `cargo test --workspace` will integrate-check it anyway.
- 2026-06-10 02:44 — CODEX REVIEW APPLIED (human-directed, doc lane only). Verified its claims
  first: migrations 1701_nav_tree + 1801_extension_query_kinds EXIST (roadmap §5 was stale →
  re-reserved RW-04→20xx, RW-06→21xx, RW-07→22xx); identifier.rs strict-allowlist precedent
  real; sink/postgres.rs interpolates quoted-but-unvalidated table/column names. Landed:
  §6 delivery-semantics contract (Source::commit() default-no-op ack hook, called post-sink-
  write; QoS sources implement for at-least-once) + added to RW-02 step 0 and the TODOs entry;
  RW-04 gains identifier-validation acceptance item + PgCopyIn-vs-BinaryCopyInWriter spike
  (sqlx already in tree; finish() caveat noted); RW-05 table discovery now via DataFusion
  catalog/SchemaProvider resolution with request-level alias→datasource authz map (manual SQL
  scraping demoted) + evaluate datafusion-federation TOGETHER with table-providers; RW-06
  spec renamed (git mv) RW-06_INSIGHTS_POLARS_RHAI.md → RW-06_INSIGHTS_ENGINE_RHAI.md +
  DummyModuleResolver explicit-import-disable + 21xx migration; roadmap §3 queue title
  updated. Declined nothing — all six points verified sound. STATUS row-6 title cell left
  for the cron (its lane); RW-02 still 🔵 mid-build, its step-0 gate check now also covers
  commit(). Committing docs only.
