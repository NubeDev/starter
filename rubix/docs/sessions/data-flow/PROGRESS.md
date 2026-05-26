# PROGRESS — data-flow scenario

Single source of truth for "which stage is done, which is next."
A new AI session reads this **first**, then opens [USAGE.md](./USAGE.md),
then opens the stage doc for the first ⏳ row below.

## How to read this file

- ✅ done — success bar in the stage doc is met, evidence recorded.
- 🚧 in progress — a session is actively working on it (don't start another).
- ⏳ next — the next stage to start.
- ⬜ blocked / waiting — earlier stage isn't ✅ yet.

A session that finishes a stage flips its row, fills the
**Date** / **Commit** / **Evidence** columns, then stops.

---

## Stage status

| # | Stage                                                    | Status | Date       | Commit | Evidence (1 line)                          |
|---|----------------------------------------------------------|--------|------------|--------|--------------------------------------------|
| 0 | Docs landed (README + 5 stage docs + USAGE + PROGRESS)   | ✅     | 2026-05-26 | _local_| `ls rubix/docs/sessions/data-flow/` has 8 files |
| 1 | [01-producer.md](./01-producer.md) — messy producer       | ⏳     |            |        |                                            |
| 2 | [02-ingest-l1.md](./02-ingest-l1.md) — L1 raw landing    | ✅     | 2026-05-26 | 9cdf4ef | 2×5min cold-restart runs: 28 rows, rate 5.6/min, 3 meters, 2 `quality='suspect'`, 0 FlowFailed |
| 3 | [03-clean-to-l2.md](./03-clean-to-l2.md) — clean to L2   | ✅     | 2026-05-26 | _local_| 2 cold-restart runs: every 1-min bucket in window has all 3 meters; `missing` present per meter; 0 `ok` rows exceed 10× median |
| 4 | [04-anomaly-rules.md](./04-anomaly-rules.md) — rules     | ✅     | 2026-05-26 | _local_ | 2×cold-restart e2e: run1 spike=10 stuck=20; run2 spike=14 stuck=21; L2 207 rows (ok=147 missing=60); L1 24 suspect rows |
| 5 | [05-dashboard-at-scale.md](./05-dashboard-at-scale.md)   | ✅     | 2026-05-26 | _local_ | 2×cold-restart e2e: L3 12 rows / 3 meters; kwh template→2 rows, litres→1 row; dashboard.data-flow-site-a loads via dashboard.get (items 2–4 follow-ups) |
| 6 | [06-scheduled-report.md](./06-scheduled-report.md)       | ✅     | 2026-05-26 | _local_ | 2×cold-restart e2e: analytics.report registered; blob_id=reports/data-flow-weekly/…html; byte_count=748–751; 5 `<tr>` tags; scheduler row next_run_at=2026-06-01 |

**Next session: stage 1 (producer) — last ⏳ row remaining.**

---

## Decisions locked so far

These were agreed when the docs landed and should not change
without a follow-up note. Each stage doc has its own
"Decisions taken" checklist for choices made during that stage.

- Scenario: 3 meters (`site-a.elec.main`, `site-a.water.main`,
  `site-a.elec.hvac`), one tenant `site-a`.
- Mess shapes: gaps, ×50 spikes, stuck-zero stretches, jitter +
  NaN — defined in [README.md §"The scenario"](./README.md#the-scenario).
- Wire shape (producer → ingest): locked in
  [01-producer.md §"Wire shape"](./01-producer.md#wire-shape-locked-both-a-and-b-emit-this).
- L1 table: `rubix.meter_readings_raw`, 14-day retention.
- L2 mart: `rubix.meter_readings_1m`, 180-day retention.
- L3 mart: `rubix.meter_readings_15m`, 730-day retention.
- Dashboard `page_id`: `dashboard.data-flow-site-a`.
- Diagnostic ids: `rubix.warehouse.meter.spike` (Warning),
  `rubix.warehouse.meter.stuck` (Error).

Per-stage open decisions (the "pick A or B" choices):

| Stage | Choice                                                 | Picked |
|-------|--------------------------------------------------------|--------|
| 01    | Flow-only producer vs extension bundle                  |        |
| 02    | Bind `rubix.warehouse.ingest` vs reuse `rule.write`    | A (bind `rubix.warehouse.ingest`; DDL via bundled migration `0003_meter_readings_raw`) |
| 03    | Periodic flow cleaner vs ClickHouse materialised view  | A (periodic flow `com.rubix.data-flow.cleaner` → `rubix.warehouse.clean_minute`; L2 DDL via bundled migration `0004_meter_readings_1m`) |
| 04    | Hardcoded Rust gate vs `rule.rhai` registry            | A (hardcoded `anomaly_gate.rs`; R-SPIKE reads L1 suspect rows, R-STUCK reads L2 same-value runs; promotion trigger — third rule — has not fired) |
| 05    | `page_set` vs `create`+`update` for dashboard build    | C (bundled JSON via `dashboards_seed.rs` — `data-flow-site-a.json`; charts are `Static` placeholders, L3 path proven via `rubix.analytics.query` templates `meter_kwh_last_24h` + `meter_litres_last_24h`) |
| 06    | Report output format + blob location                    | HTML via `FsBlobStore` at `RUBIX_BLOB_ROOT` (default `/tmp/rubix-blobs`); templates `meter_kwh_site_a_weekly` + `meter_litres_site_a_weekly` (self-contained, hard-code `tenant_id='site-a'`); undo path deferred |

---

## Follow-up notes (spillover)

Anything that didn't fit in a stage's "Success bar" lives here as
its own session note. Add a row when you create the note; never
rewrite a stage doc to absorb spillover.

| Date | Stage | Note | Status |
|------|-------|------|--------|
| 2026-05-25 | 01 | [tool-call bridge + producer e2e](./2026-05-25-data-flow-01-producer-tool-call-bridge.md) | superseded — scheduler is fine; see 2026-05-26 note for actual blockers |
| 2026-05-26 | 01 | [scheduler verified; synth fires 2–3× per cron tick](./2026-05-26-data-flow-01-producer-multi-fire.md) | open — duplicate-fire bug in tool-call seed adapter + bar #2 math vs 60s cron mismatch |
| 2026-05-26 | 02 | [two boot-wiring blockers (in-memory CH writer + seed adapter racing tool_input)](./02-ingest-l1-blockers-2026-05-26.md) | resolved by the B1+B2 fixes that landed in working tree before stage 02 e2e |
| 2026-05-26 | 02 | [cadence + bar reconciliation (60s scheduler claim cadence)](./02-ingest-l1-cadence-and-bars-2026-05-26.md) | resolved — bar #1 rewritten as rows/min rate, bar #3 documents `DATA_FLOW_SPIKE_PROB` override for stage-close validation |
| 2026-05-26 | 05 | [chart resolver ↔ analytics path + zoom](./2026-05-26-data-flow-05-followups.md) | resolved (charts + L2 cross-over data path) — `ChartSource::AnalyticsTemplate` (`ea68458`) + L2 template `meter_value_24h_1m` and side-by-side L2 row on `data-flow-site-a`; live resolve shows L3 7pts vs L2 43–49pts per meter. Interactive zoom (same chart node swapping L2↔L3 on user window change) remains a separate follow-up. |

Naming convention (matches the rest of `docs/sessions/`):

```
rubix/docs/sessions/data-flow/<stage-NN>-<short-topic>-YYYY-MM-DD.md
```

Use the template at [_SESSION-TEMPLATE.md](./_SESSION-TEMPLATE.md).

---

## How to update this file (checklist for the finishing session)

1. **Live e2e is mandatory.** Do not flip a row to ✅ on the
   strength of unit tests, `cargo test`, or `HTTP 200` alone.
   The stack must be running from your built binary, you must
   drive the verbs in the stage doc, and you must inspect
   ClickHouse / Postgres directly per
   [USAGE.md §5](./USAGE.md#5-inspecting-state-directly-skip-the-verb)
   to confirm every bullet in the success bar. Repeat with a
   cold restart; both runs must pass.
2. Flip the stage's status emoji to ✅.
3. Fill **Date** (ISO `YYYY-MM-DD`), **Commit** (`git rev-parse --short HEAD`),
   and a one-line **Evidence** snippet from the **live** run
   (e.g. `count(*)=237 in rubix.meter_readings_raw after 5min`),
   not from a unit test.
4. Bump the next ⬜ row to ⏳.
4. Tick the per-stage decision under "Per-stage open decisions".
5. Add any spillover rows to "Follow-up notes".
6. Commit with message
   `docs(data-flow): stage NN done — <one-line summary>`.
