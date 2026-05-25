# Stage 04 — anomaly rules (spikes + stuck zeros)

## Scope

**In:** two rules that read L2 (`rubix.meter_readings_1m`) and
fire `rubix.alert.send` when they trip. The two rules:

- **R-SPIKE** — a clipped row appears (`quality='clipped'`). The
  spike already happened; the alert tells the operator the
  cleaner had to intervene.
- **R-STUCK** — a meter has ≥ 5 consecutive `quality='stuck'`
  buckets. Sensor likely frozen.

**Out:** machine-learning anomaly detection (out of scope until
the deterministic rules above prove the dispatch path); alert
delivery to Slack / email (the alert sink design defers that —
see [design/insights/README.md](../../design/insights/README.md)).

## Where the rules live

Pick **one** and lock it. The rubix insights design
explicitly endorses both with a clear promotion trigger.

- **A. Hardcoded Rust check** in a new
  `rubix-tools/src/warehouse/anomaly_gate.rs`, mirroring
  `system::disk::run_insights_gate`. The gate is called by the
  cleaner flow (stage 03) on every materialisation tick.
  **Preferred for v0** — one rule already wins per the insights
  doc's logic ("one rule does not justify a rule engine"); two
  rules at the same call site (the cleaner tick) still don't.
- **B. `starter-insights::RuleRegistry` + `rule.rhai`** —
  proper rule engine. Pick this only if a third rule appears
  during this stage (e.g. operator wants a per-meter threshold);
  that is the explicit promotion trigger from the insights doc.

Default: **A**. Migration to B is the insights doc's documented
path — no rework of the dispatch shape `(AlertSeverity,
Diagnostic) → alert_send::dispatch`.

## Diagnostics (locked, both A and B emit these)

| Diagnostic id                       | Severity | Params                                        |
|-------------------------------------|----------|-----------------------------------------------|
| `rubix.warehouse.meter.spike`       | Warning  | `meter_id`, `bucket_start`, `clipped_to`      |
| `rubix.warehouse.meter.stuck`       | Error    | `meter_id`, `stuck_since`, `bucket_count`     |

Add the matching MessageKey entries to **both** `rubix-spi/
catalogues/en.json` and `es.json` in the same commit (workspace
rule R5 — see flow-programmer / clickhouse-rules docs).

## Pre-flight

- Stage 03 success bar green — L2 has `clipped` and (at least
  occasionally) `stuck` rows.
- The alert sink is wired to *something* observable —
  v0 dispatches to the tracing pipeline, so
  `RUST_LOG=info,rubix_tools=debug` is enough to see fires.
  Don't expect Slack.

## Steps (shape A — hardcoded gate)

1. Add `anomaly_gate.rs` next to `system/disk/run_insights_gate`.
   Two functions, both `pub(crate) async fn`:
   `check_spike(rows: &[CleanedRow]) -> Vec<Diagnostic>` and
   `check_stuck(rows: &[CleanedRow]) -> Vec<Diagnostic>`.

2. Call both from the cleaner flow's post-write hook, dispatching
   each returned diagnostic through the same `alert_send::dispatch`
   the disk gate uses.

3. Add the two MessageKey entries to both catalogue files.

4. Test in isolation:

   ```bash
   cargo test -p rubix-tools warehouse::anomaly_gate
   ```

   The test pre-builds a `Vec<CleanedRow>` with one clipped row
   and one 5-bucket stuck stretch, asserts both diagnostics fire
   exactly once each.

5. End-to-end smoke: run the producer for 10 minutes with
   `DATA_FLOW_SPIKE_PROB=0.05` and `DATA_FLOW_STUCK_PROB=0.05`
   (forced high so the gate definitely fires). Grep the agent
   log for the diagnostic ids.

## Success bar

Stage 04 is done when **all three** are true:

1. The unit test in step 4 passes (each gate fires exactly once
   for its fixture, zero times for the other's fixture).
2. The 10-minute high-mess smoke run logs **at least one** of
   each diagnostic id.
3. `alert_send::dispatched_count()` (the same counter the disk
   insights test uses — see
   [design/insights/README.md](../../design/insights/README.md))
   increases by the number of diagnostics fired in the smoke
   run, ±0.

## If it fails

In order, check:

1. **No diagnostics fire even with forced high mess** — the gate
   isn't being called from the cleaner flow. Add a `tracing::info!`
   at the gate entry; if it doesn't log, the wiring is wrong.
2. **Diagnostics fire but `dispatched_count` doesn't move** — the
   alert sink rejects the diagnostic. Same failure mode as the
   v0 disk gate; cross-reference the insights doc's dispatch
   boundary.
3. **Stuck rule false-positives on cold-start** — the producer
   hasn't run long enough for the cleaner to see ≥ 5 stuck
   buckets in a row. Wait 10 minutes after starting the
   producer, not 5.

Write follow-up notes as `YYYY-MM-DD-data-flow-04-rules-<topic>.md`
and stop.

## Decisions taken

- [ ] Shape A (hardcoded gate)  /  [ ] Shape B (`rule.rhai`)
- Spike diagnostic id: `rubix.warehouse.meter.spike`
- Stuck diagnostic id: `rubix.warehouse.meter.stuck`
