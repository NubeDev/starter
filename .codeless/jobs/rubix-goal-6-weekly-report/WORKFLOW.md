# Workflow — rubix-goal-6-weekly-report

## Sequencing

15 stages across five phases. Strict dependency order: A (upstream starter-cron + migration + trigger_schedule body) → B (upstream FlowAsService + tick) → C (rubix analytics verbs + templates) → D (wire Goal 6 + integration test) → E (closing docs + PR). Five REVIEW gates total.

This job is upstream-heavy by design — R2 strictly. Phase A and B touch starter, not rubix. The cron primitive and FlowAsService are starter-wide infrastructure; rubix is the first consumer. If a follow-up reveals other future consumers need different shapes, that's a separate ticket — this job ships what rubix's Goal 6 needs.

## Per-stage discipline

### Phase A — upstream cron primitives

The cron primitives are pure-ish: parser is pure, migration is sql, trigger_schedule body is small. Discipline:

1. **starter-cron is a tiny crate.** Resist the urge to add helpers, observability, retries, fancy iter APIs. One pub fn (`next_fire`), one Error enum, four unit tests. If it grows beyond ~150 LoC total, something's wrong.
2. **The cron crate from crates.io is the parser.** Don't handwrite a cron grammar. Pin a version; the wrapper API is what we own.
3. **The migration must include the pg_notify trigger.** Don't split into two migrations — the table without the trigger is incomplete.
4. **trigger_schedule.rs stays passive.** The NodeBehavior reads cron_expr from config, exposes it via the output slot, and **does not fire anything**. Firing is Phase B's tick. Mixing them couples the node to the scheduler in a way that breaks the "everything is a node" invariant.
5. `cargo test -p starter-cron` and `cargo test -p starter-store-postgres --features testcontainers` and `cargo test -p starter-flow-nodes --test trigger_schedule_test` green per stage.
6. `./rubix/scripts/lint-doc-refs.sh` clean — no SCOPE / docs/scope / docs/sessions references in code comments.

### Phase B — FlowAsService

The cron-aware companion to FlowAsTool. The two big design constraints:

1. **`SELECT FOR UPDATE SKIP LOCKED` is non-negotiable.** Two rubix-agent processes must never double-fire a schedule. The test for this lives in `scheduled_flows_tick_test.rs` and uses two independent PG pools to simulate two instances.
2. **The Clock trait is non-negotiable too.** The tick test must drive time via TestClock; running the test against wallclock would either be flaky (60s ticks) or impossible (have to wait for actual cron deadlines). Every place the service consults "now" goes through `clock.now()`, never `chrono::Utc::now()`.
3. The service.rs file is allowed to be larger (~250 lines) because it carries state + register + tick + start. If it crosses 400, split tick into `service/tick.rs` per R1.
4. `cargo test -p starter-flow-surfaces --features testcontainers` green; doc-refs lint clean.

### Phase C — rubix analytics verbs

The two verbs are where rubix consumes the upstream primitives. Discipline:

1. **Templates are include_dir!-bundled .sql files.** Same pattern as `rubix-flows/flows/`. Hand-edit forbidden post-bundle; new template = new file = new commit (in future work).
2. **Parameter binding uses ClickHouse's named-param syntax.** `{name:Type}` is parameterised correctly by the CH client; string interpolation into SQL is forbidden (SQL injection vector). The integration test asserts a malicious param value (`'; DROP TABLE x; --`) lands as a literal string, not as SQL.
3. **analytics.report's Reversible = blob delete.** Same Reversible registry that Goals 2/3/4 use; same `undo_snapshots` semantics. The revert deletes the blob; the snapshot row carries the blob_id for revert lookup.
4. **MessageKeys in both en.json + es.json same commit.** R5, every time.
5. `cargo test -p rubix-tools --features testcontainers` green per stage.

### Phase D — wire Goal 6 end-to-end

The integration phase. Three commits, each load-bearing:

1. **D.1 deletes WeeklyReportStub.** Don't comment-out; delete. The test that referenced it is replaced with goal_6_weekly_report_test.rs in D.3.
2. **D.2 wires FlowAsService at boot.** This is the line that activates the durable scheduler. If FlowAsService::start panics (e.g. PG pool not ready), the agent must surface a clear error — wrap the spawn in a Result and let main fail loud rather than silently running without a scheduler.
3. **D.3's integration test must drive the FlowAsService clock.** No wallclock waits in CI. The test injects a TestClock, advances 7 days, asserts the fire happens. Same Clock-trait discipline as Phase B.
4. The flow YAML rewrite (D.1) must keep flow_id stable (`com.rubix.weekly-report`). Operators who scripted against the prior MCP catalogue continue to work; only the body changes, not the surfaced tool name.

### Phase E — closing

One stage, three artifacts:

1. **THIN-SLICE.md table** — flip Goal 6 row to "real" with the evidence link. Only Goal 1 remains stubbed; document the unblock criteria.
2. **Design doc + session note + starter-changes ledger entries** — present-tense, mirroring the goals-2-4-3 closing pattern.
3. **PR** — `gh pr create` only after operator confirmation. Title: `feat: goal 6 weekly-report end-to-end + durable cron scheduler upstream`.

## Anti-patterns specific to this job

- **Don't skip Phases A/B for a faster rubix-only ship.** The whole point is the upstream cron primitive becomes available for every future scheduled flow. Skipping = R2 violation = future tech debt.
- **Don't fold starter-cron into starter-flow-spi prematurely.** The crate boundary is what makes the cron primitive reusable outside flow context (e.g. a future scheduled-job binary). If the crate genuinely is < 100 LoC after a thoughtful pass, fine — but default is separate.
- **Don't add a REST surface for schedule management this job.** Out of scope. Operators edit `scheduled_flows` via SQL or the future REST surface in a follow-up.
- **Don't add retry-on-failure to the tick.** Out of scope. `last_run_status = "failed"` is the operator signal.
- **Don't add PDF rendering.** Out of scope. `format=pdf` returns `rubix.analytics.report.format_unsupported`.
- **Don't load all six templates in one giant SQL file.** Each template = one .sql file = one verb-file-equivalent. R1 applies to data files too.
- **Don't list paths with brace expansion in handovers.** Trips diff-verify.
- **Don't list a path under Done that the stage didn't modify.** Same trap.
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Each REVIEW gate commits and pushes the stage(s) that led to it; the gate itself commits nothing. Write the gate's question into `handover.md` for the next stage, halt, wait for operator confirmation.

At each REVIEW gate, the handover must include:

- One-line title per commit made in the phase, with the file count touched.
- `cargo test` summary per crate.
- For Phase B's gate: explicit confirmation that `SELECT FOR UPDATE SKIP LOCKED` is correctly used (multi-instance safe) and `TestClock` drives the tick test (no wallclock).
- For Phase C's gate: one operator-runnable manual flow (`curl tools/call analytics.query disk_history_weekly` → JSON rows; `curl tools/call analytics.report` → blob URL).
- For Phase D's gate: one operator-runnable manual flow demonstrating boot → trigger-now (or wait for scheduled fire) → blob lands → undo deletes blob.
- Any deviation from SCOPE.
- Whether the upcoming phase is unblocked.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-goal-6-weekly-report`.

A stage is not done until all three are green and the push succeeds. REVIEW gate stages mark `git` as `skipped — gate-only`. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. ≤ 400 lines hard, ~100 typical.
- Code comments link `docs/design/<area>/README.md` only.
- No phasing markers in code.
- Upstream-first (R2). starter-cron / starter-store-postgres / starter-flow-nodes / starter-flow-surfaces all land before rubix consumes them.
- Tool outputs are `Diagnostic` + structured data, never pre-formatted strings.
- Catalogue files are the source of truth for MessageKeys. No new key without entries in both en.json and es.json in the same commit.
- Tests live with the code in the same commit (R6).
- Comments explain *why*, not *what*. No emojis.

## References

- `crates/starter-flow-nodes/src/trigger_schedule.rs` — the 23-line stub Phase A fills.
- `crates/starter-flow-surfaces/` — where FlowAsService lands.
- `crates/starter-export/{html,csv_backend,json_backend,pdf}.rs` — the report rendering primitives.
- `crates/starter-blob-fs/` — the dev-default blob backend.
- `rubix/crates/rubix-tools/src/analytics/` — the verb home.
- `rubix/crates/rubix-flows/flows/weekly-report.yaml` — the flow that becomes real.
- `rubix/docs/design/starter-changes/README.md` — upstream PR ledger.
- `rubix/docs/scope/GAPS.md` row 16 — the gap this job closes.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — the verification pattern to mirror.
- `rubix/SCOPE.md` — R1–R13.
- `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
