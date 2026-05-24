# Scope — rubix-goal-6-weekly-report

## Goal

Light up Goal 6 (weekly-report) end-to-end through the now-real agent loop. This means: a `trigger: schedule` flow fires on a cron expression backed by a durable scheduler, an `ai-agent` node dispatches `analytics.query` against a named ClickHouse template, the results are rendered through `starter-export` into a report blob, the blob is persisted via `starter-blob-fs` (default) and listed/fetchable via the existing blob route surface. The current stub `WeeklyReportStub` at `rubix/crates/rubix-tools/src/analytics/weekly_report.rs` is deleted; the bundled `flows/weekly-report.yaml` becomes a real scheduled flow.

The job is **upstream-first heavy** per R2 — the durable scheduler and the `FlowAsService` schedule trigger do not exist yet in starter (the `trigger_schedule.rs` file in `starter-flow-nodes` is a 23-line stub with only KIND_ID + descriptor). Both upstream pieces land in starter **first**, with their own integration tests against `MockNodeBehavior` and a fake clock, before the rubix consumer code lands. This pattern matches every prior rubix job that needed an upstream lift (R2; see `docs/design/starter-changes/`).

After this job ships:
- **5 of 6 SCOPE goals real** — only Goal 1 (dashboards SDUI) remains stubbed.
- **Cron primitive available for every future scheduled flow** — alerts on schedule, retention sweeps, periodic reports.
- **Analytics query primitive available** — any future rubix verb that needs to query ClickHouse safely has a named-template path.
- **Blob/export primitive wired** — any future verb that needs to produce a downloadable artefact has a worked example.

## In scope

### Phase A — upstream durable scheduler in starter

The durable scheduler lives in starter, not rubix. It is the missing piece behind `trigger_schedule.rs`'s KIND_ID. Concrete pieces:

- **`crates/starter-cron`** — new tiny crate that parses cron expressions and computes `next_fire(now: DateTime<Utc>, expr: &str) -> DateTime<Utc>`. Pure-function API, no I/O. Use the `cron` crate from crates.io as a dependency (well-vetted, no transitive bloat) rather than handwriting a parser; the rubix-side wrapper is what we own. Unit tests cover the standard cron forms (`0 0 * * 0` weekly, `*/15 * * * *` every 15 minutes, `0 9-17 * * 1-5` business hours, plus a malformed-expression error path).
- **`crates/starter-store-postgres/migrations/<NNNN>_scheduled_flows.sql`** — new dimension table:
  - `id ULID PK`
  - `tenant_id`
  - `flow_id TEXT` (e.g. `com.rubix.weekly-report`)
  - `cron_expr TEXT`
  - `next_run_at TIMESTAMPTZ` (computed via `starter_cron::next_fire` at insert and after each fire)
  - `last_run_at TIMESTAMPTZ NULL`
  - `last_run_status TEXT NULL` (`succeeded` / `failed` / `cancelled`)
  - `last_run_message TEXT NULL` (the failure summary if any; truncated to 4 KB)
  - `enabled BOOL NOT NULL DEFAULT TRUE`
  - `created_at`, `created_by`
  - UNIQUE `(tenant_id, flow_id)` — one schedule per `(tenant, flow)`.
- **`pg_notify('starter_scheduled_flows', ...)`** trigger on insert/update so consumers (rubix-agent) can react to schedule changes without polling the table for the data itself.
- **`starter-flow-nodes::trigger_schedule`** — implement the `NodeBehavior` for the existing stub. Today it's KIND_ID + descriptor only; this phase adds the `invoke` body that reads the cron expression from the node's config, computes `next_fire`, and returns. The node itself is **passive** — it's the entry point; the actual firing comes from the FlowAsService tick in Phase B.

### Phase B — `FlowAsService::schedule` in starter-flow-surfaces

The cron-aware companion to `FlowAsTool`. The current `starter-flow-surfaces` crate has a `flow_registry` module; this phase adds a `service` module.

- **`crates/starter-flow-surfaces/src/service.rs`** — new module (verb file ≤ 400 lines):
  - `pub struct FlowAsService` holding a `Pool<Postgres>`, an `Arc<FlowRegistry>`, an `Arc<dyn FlowRunner>`, an `Arc<dyn Clock>` (so tests can drive time deterministically).
  - `pub fn register_schedule(tenant_id, flow_id, cron_expr) -> Result<()>` — writes a row into `scheduled_flows`, computes `next_run_at`, emits a `pg_notify`.
  - `pub fn unregister_schedule(tenant_id, flow_id)` — sets `enabled = false`.
  - `pub fn tick(&self) -> Result<usize>` — runs once per 60s. Claims rows where `enabled AND next_run_at <= clock.now()` via `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 32`. For each claimed row: dispatches the flow via `FlowRunner::run(flow_id, ...)`, awaits completion (or wraps with a watchdog timeout), writes `last_run_at`, `last_run_status`, `last_run_message`, and recomputes `next_run_at` via `starter_cron::next_fire`. The `SKIP LOCKED` clause is what makes this multi-instance safe — two rubix-agent processes won't double-fire a schedule.
  - `pub fn start(self) -> JoinHandle<()>` — spawns the 60s tick loop as a tokio task.
- **Integration test** — `tests/scheduled_flows_tick_test.rs` against testcontainers Postgres: register a schedule that fires "every minute", advance a fake clock by 2 minutes, assert tick fires twice, assert `last_run_at` advances correctly.
- **`crates/starter-flow-surfaces/Cargo.toml`** — depends on `starter-cron`, `sqlx`, `tokio`.

### Phase C — rubix analytics verbs (`query` + `report`)

Two verbs in `rubix/crates/rubix-tools/src/analytics/` replace today's stubs:

- **`analytics/query.rs`** — `AnalyticsQueryTool`. Inputs: `{ name: String, params: Map<String, Value> }`. Looks up a named SQL template under `rubix-tools/src/analytics/templates/<name>.sql` (compiled in via `include_dir!` like the bundled flows). Binds params safely using ClickHouse's named parameter syntax (`{name:Type}`) so the SQL is parameterised, not concatenated. Runs against `ChClient` (already threaded through the tool registry per PR #31), returns `{ rows: Vec<Value>, row_count: u32 }`. Read-only; no `Reversible`. Six bundled templates ship with this job:
  - `disk_history_weekly.sql` — `SELECT toStartOfDay(at) AS day, avg(percent_used) AS avg_percent, max(percent_used) AS peak_percent FROM system_disk_history WHERE at >= now() - INTERVAL 7 DAY GROUP BY day ORDER BY day`
  - `alert_count_weekly.sql` — count of `alert.send` calls in last 7 days from `changelog`
  - `flow_run_summary_weekly.sql` — flow runs grouped by status from the flow audit history
  - `user_activity_weekly.sql` — distinct active users per day from `changelog.actor_id`
  - `clickhouse_writes_weekly.sql` — count of `clickhouse.*` mutating calls
  - `undo_count_weekly.sql` — count of `rubix.undo.last` invocations
- **`analytics/report.rs`** — `AnalyticsReportTool`. Inputs: `{ template: String, queries: Vec<{ name, params }>, format: "html" | "json" | "csv" }`. Runs each named query, collects the rows, hands them to `starter-export` for rendering. `format=html` → `starter-export::html`, `format=csv` → `starter-export::csv_backend`, `format=json` → `starter-export::json_backend`. Writes the rendered bytes to a blob via `starter-blob-fs::write` (the dev binary uses fs; production picks s3/garage via config). Returns `{ blob_id, url, byte_count, format }`. Read-only — but the blob it produces is persistent; `Reversible` is appropriate (revert = `starter-blob-fs::delete(blob_id)`).
- **MessageKeys** — `rubix.analytics.query.ran`, `rubix.analytics.query.unknown_template`, `rubix.analytics.query.bind_error`, `rubix.analytics.report.rendered`, `rubix.analytics.report.empty`, `rubix.analytics.report.format_unsupported`. EN + ES in the same commit.
- **Skill update** — `rubix-skills/skills/analytics-reporter/SKILL.md` present-tense — the two verbs, the named-template contract, the report formats.

### Phase D — wire Goal 6 end-to-end

- **`rubix-tools/src/analytics/weekly_report.rs`** — **delete**. The stub is no longer needed.
- **`rubix-tools/src/analytics/mod.rs`** — drop the stub export; add `query`, `report`.
- **`flows/weekly-report.yaml`** — rewrite. Replace `trigger: explicit` with `trigger: schedule` and add `cron_expr: "0 8 * * 1"` (Monday 08:00 UTC). Replace `allowed_tools: [com.rubix.weekly-report]` with `allowed_tools: [analytics.query, analytics.report, rubix.alert.send, rubix.undo.last]`. The agent's job is now: call the six named queries → call report with format=html → optionally call `alert.send` if a query signals a degradation pattern.
- **`rubix-agent::main`** — wire `FlowAsService::start` at boot for any flow whose YAML carries `trigger: schedule`. The seed path (mirroring `flows_definitions` from PR #32): on first boot, every bundled YAML with a schedule trigger gets a row in `scheduled_flows`.
- **Integration test** — `rubix-agent/tests/goal_6_weekly_report_test.rs`:
  1. Boot the agent against testcontainers PG + CH with the weekly-report schedule registered.
  2. Pre-populate `system_disk_history` with synthetic data covering 7 days.
  3. Advance the FlowAsService clock by 7 days to trigger one fire.
  4. Assert the flow runs to completion, the agent's reply is non-empty, an HTML report blob lands via `starter-blob-fs`, the blob bytes contain expected substrings (a table row per day from the disk-history template).
  5. Call `rubix.undo.last` → assert the blob is deleted.
- **`docs/design/reports/README.md`** — rewrite from placeholder to present-tense covering the schedule contract, the query template registry, the report format options, the blob backend choice, the undo contract for reports.

### Phase E — closing docs + smoke + PR

- **`THIN-SLICE.md` "Goals lit up" table** — flip Goal 6 row from **stubbed** to **real** with the evidence link. Only Goal 1 remains stubbed.
- **Closing session note** `docs/sessions/<today>-goal-6-landed.md` — one operator-runnable manual flow (curl tools/call weekly-report → assert blob lands → curl the blob URL → see the rendered HTML).
- **`docs/design/starter-changes/`** — add entries for the three upstream pieces landed (`starter-cron`, `scheduled_flows` migration, `FlowAsService`) with their commit references.
- **`docs/scope/GAPS.md` row 16** — flip to "addressed in this job — see `docs/design/scheduling/README.md`" (new design doc covering the scheduler architecture).
- **PR** — one PR off `codeless/rubix-goal-6-weekly-report` with phase-by-phase commits, reviewed in order.

## Out of scope

- **Goal 1 (dashboards SDUI).** Still a separate future job.
- **Per-tenant schedule isolation beyond the `tenant_id` column.** Multi-tenant CH isolation remains a Phase 4 entry-gate concern.
- **Free-form SQL via `analytics.query`.** Templated queries only (per design-call answer). Free-form support is a follow-up if real demand surfaces.
- **Web push / email delivery of the report.** The report lands as a blob with a URL; notification delivery is a future concern.
- **PDF export.** `starter-export::pdf.rs` exists but PDF rendering is non-trivial; `format=pdf` returns `rubix.analytics.report.format_unsupported` in v1 and is tracked as a follow-up.
- **Schedule expression UI / management endpoints.** `register_schedule` and `unregister_schedule` are called from boot (seeding bundled flows) only this job. A REST surface for managing schedules at runtime is a follow-up. Operators can edit `scheduled_flows` directly via SQL for now.
- **Cron expression validation in `flow.deploy` / `flow.lint`.** Phase D adds a TODO in `flow_ops/lint.rs` referencing the future check; doing it properly requires the lint to depend on `starter-cron`, which is fine, but the work isn't in this job.
- **Sub-minute schedules.** Tick granularity is 60s; cron expressions implying finer granularity (which standard cron doesn't support anyway) silently round up to the next 60s boundary.
- **Live LLM in CI.** Recorded fixtures under `rubix-agent/tests/fixtures/` remain the seam.
- **No `--no-verify`, no `--force` push.** No phasing markers in code.

## Constraints

- **R1 — One verb per file.** ≤ 400 lines hard, ~100 typical. Each new file in `starter-cron`, `starter-flow-surfaces::service`, `rubix-tools::analytics::*` obeys.
- **R2 — Upstream-first.** Phases A and B land **before** Phases C and D. Within Phase A, the migration + `starter-cron` crate land before `starter-flow-nodes::trigger_schedule` consumes them.
- **R3 — Doc-tier rule.** Code comments link `docs/design/<area>/README.md` only. `./rubix/scripts/lint-doc-refs.sh` enforces it on every stage.
- **R4 — Tool outputs are `Diagnostic` + structured data**, never pre-formatted strings.
- **R5 — Catalogue files are the source of truth for MessageKeys.** Every new code needs entries in both `en.json` and `es.json` in the same commit.
- **R6 — Tests live with the code in the same commit.**
- **R10 — Reverse-DNS ids.** The schedule trigger keeps its `starter.flow.trigger.schedule` KIND_ID.
- **Commit messages.** `feat(starter-cron):` for the new crate, `feat(starter-store-postgres):` for migrations, `feat(starter-flow-nodes):` for trigger_schedule body, `feat(starter-flow-surfaces):` for FlowAsService, `feat(rubix-tools):` for analytics verbs, `feat(rubix-flows):` for the YAML rewrite, `feat(rubix-agent):` for the boot wiring, `docs:` and `chore(docs+ci):` as appropriate.
- **Per-phase REVIEW gate.** A → B → C → D → E each end with REVIEW. Five gates total.

## Open questions

1. **`starter-cron` or `starter-store-postgres::scheduler`?** The cron crate is pure-function; the PG table is in the store crate. Default: separate crates per current starter conventions; if review reveals the cron crate is too small to justify its own Cargo manifest (< 100 lines total), fold it into `starter-flow-spi` instead. Phase A.1 decides after the cron parser lands.
2. **Tick granularity 60s vs 30s.** Default 60s — matches standard cron's lowest expressible unit. If a future use case needs faster, that's a config knob, not a redesign.
3. **`Reversible` semantics for `analytics.report`.** The report blob is the side effect. Revert = delete the blob via `starter-blob-fs::delete`. But: if the report is referenced elsewhere (a future email / dashboard link), the delete cascades to a broken link. Default: register `Reversible`, document the cascade caveat in `docs/design/reports/README.md`, and rely on the operator to undo within the snapshot retention window (90 days per the goals-2-4-3 `undo_snapshots` shape).
4. **Should `FlowAsService::tick` run inside the rubix-agent process or as a separate binary?** Default: in-process tokio task in the existing rubix-agent binary. Separating into its own binary is the right move once we have a scheduler-fleet concern; not in this job.
5. **What about retries on `FlowRunner::run` failure inside the tick?** Default: no automatic retry. `last_run_status = "failed"`, `last_run_message = <error>`, `next_run_at` advances to the next cron tick. Operator sees failures in `last_run_status` and can re-trigger manually via a yet-to-exist REST surface (tracked as a follow-up). If retry-on-failure is needed, it's a config knob added later.
6. **Cron expression timezone.** Default: UTC. The cron expression in `flows/weekly-report.yaml` says "Monday 08:00 UTC". A future "schedule timezone per tenant" capability lives outside this job.

## References

- `crates/starter-flow-nodes/src/trigger_schedule.rs` — the 23-line stub Phase A fills.
- `crates/starter-flow-surfaces/` — where FlowAsService lands.
- `crates/starter-export/` — the report rendering primitives (html, csv, json available; pdf deferred).
- `crates/starter-blob-fs/` — the dev-default blob backend.
- `rubix/crates/rubix-tools/src/analytics/` — the verb home.
- `rubix/crates/rubix-flows/flows/weekly-report.yaml` — the flow that becomes real.
- `rubix/docs/design/starter-changes/README.md` — upstream PR ledger; three new entries land in Phase E.
- `rubix/docs/scope/GAPS.md` row 16 — the gap this job closes.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — verification pattern to mirror.
- `rubix/SCOPE.md` — R1–R13.
- `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
