# 02 — wiring is green, bars are still scheduler-cadence-broken

> Session note. Live e2e against a freshly-built `rubix-agent` from
> `master` (with the two blocker-fixes the user landed earlier
> today). The stage-02 *wiring* — DDL on boot, synth → ingest data
> link, `ChClient`-backed ingest — is correct and rows are landing
> with the right shape and the right meters. **But two of the
> three success-bar bullets in
> [`02-ingest-l1.md`](./02-ingest-l1.md#success-bar) are unreachable
> in a 5-minute window under the durable scheduler's real claim
> cadence.** This is the same shape of bar-vs-reality mismatch
> already documented for stage 01 in
> [`2026-05-26-data-flow-01-producer-multi-fire.md`](./2026-05-26-data-flow-01-producer-multi-fire.md),
> not a regression from the blocker fixes.
>
> No code changed. PROGRESS.md stays with stage 02 ⬜.

---

## Context

- **Stage:** 02 — raw landing in ClickHouse (L1).
- **Started from:** commit `3c10414` + the user's blocker-fix
  changes on top (still working-tree, not yet committed at the
  time of this note).
- **Trigger:** ran the stage doc's e2e drive after a `make
  restart`; rows land, but the success-bar bullets ≥200 and
  `suspect > 0` cannot meet their thresholds in 5 min under the
  current scheduler cadence.

## E2E run — what actually happened

Agent boot, restart, login per
[USAGE.md §1–§2](./USAGE.md#1-bring-the-stack-up):

```
[2026-05-26T01:10:17.108Z] rubix.boot              rubix migrations applied sources=7
[2026-05-26T01:10:17.581Z] rubix.boot              rubix ClickHouse migrations applied
[2026-05-26T01:10:17.735Z] rubix_agent::health     rubix-agent listening bind=0.0.0.0:8088
[2026-05-26T01:10:17.674Z] rubix.boot.scheduler    seeded scheduled flow flow_id=com.rubix.data-flow.producer cron_expr="*/5 * * * * *" next_run_at=2026-05-26 01:10:20 UTC
```

Confirmation the L1 table landed via the new bundled migration
(not via `rubix.clickhouse.rule.write`):

```bash
$ curl -s -X POST -d "SHOW TABLES FROM rubix" http://127.0.0.1:8124/
_starter_ch_migrations
documents
entities_dict
events
meter_readings_raw      ← new
raw_events
samples
system_disk_history
```

Smoke against the registered ingest tool (no rows — just shape):

```bash
$ curl -s -b /tmp/smoke-cookies.txt -X POST \
    http://127.0.0.1:8088/api/v1/tools/rubix.warehouse.ingest \
    -H 'content-type: application/json' -d '{"rows":[]}'
{"inserted":0,"summary":{"code":"rubix.warehouse.ingest.empty"},"written_at_ms":...}
```

After ~10 minutes of the producer flow running:

```bash
# Bar 1 — count >= 200
$ curl -s -X POST -d "SELECT count() FROM rubix.meter_readings_raw" http://127.0.0.1:8124/
58

# Bar 2 — three distinct meter_id values
$ curl -s -X POST -d "SELECT meter_id, count() FROM rubix.meter_readings_raw GROUP BY meter_id ORDER BY meter_id FORMAT TSV" http://127.0.0.1:8124/
site-a.elec.hvac    20
site-a.elec.main    18
site-a.water.main   20

# Bar 3 — countIf(quality='suspect') > 0
$ curl -s -X POST -d "SELECT countIf(quality='suspect'), countIf(quality='ok'), countIf(quality='missing') FROM rubix.meter_readings_raw FORMAT TSV" http://127.0.0.1:8124/
0   58   0
```

Synth-stat fires from the agent log (aggregated across the ~10
min):

```
11 × stuck_active=0
11 × spikes=0
11 × nans=0
10 × gaps=0
10 × emitted=3
 1 × gaps=1
 1 × emitted=2
```

So **11 synth fires in ~10 min** — one per minute, matching the
durable scheduler's claim cadence, not the YAML's `*/5 * * * * *`.
Each fire emits 3 rows (one gap-tick emitted 2). Yet the CH table
holds **58 rows**, ~ 1.7× the 32 the log would predict. That's
the duplicate-fire residue from the seed adapter / engine
interaction documented in
[`2026-05-26-data-flow-01-producer-multi-fire-root-cause.md`](./2026-05-26-data-flow-01-producer-multi-fire-root-cause.md);
it is not introduced by the stage-02 wiring.

## What the e2e proves (✅) and does not (❌)

✅ **Wiring works end-to-end:**

- `rubix.warehouse.ingest` is registered (`HTTP 200` on
  `/api/v1/tools/rubix.warehouse.ingest`).
- L1 DDL lands at boot via the new bundled migration
  `0003_meter_readings_raw/up.sql` — independent of the
  `rubix.clickhouse.rule.write` verb path.
- The seed-adapter blocker is fixed: synth's rows reach
  `ingest.input` via the `synth.output → ingest.input` link,
  the YAML's `tool_input: {}` no longer races. Every row in CH
  has all seven columns populated and a sensible `quality`
  value.
- Three distinct `meter_id`s present, monotonic `value` (per the
  `value` column rising across the run), `epoch_ms` populated.
- Zero `FlowFailed`, zero panics, zero `UnknownTool` errors in
  the agent log for the producer flow's run lineage.

❌ **Two success-bar bullets remain unmet:**

- **Bar 1 — `count >= 200` after 5 min** is unreachable under
  the real scheduler cadence. 60-s claim interval × 3 meters ×
  5 min = 15 emitted-row max minus gaps; multi-fire residue
  pushes it 1.5–2× higher, so ~25–35 rows in CH after 5 min is
  the honest ceiling. We saw 58 over ~10 min, which is
  consistent.
- **Bar 3 — `countIf(quality='suspect') > 0`** is
  probability-bounded: default `spike_prob = 0.005` over ~30
  emitted rows in a 5-min window predicts ~0.15 spikes — coin-
  flip whether any fire at all. After 10 min and ~32 emitted
  synth rows, none fired (within noise). Saying "the spike path
  works" requires either (a) many more rows, or (b) a one-shot
  knob override at boot via `DATA_FLOW_SPIKE_PROB=0.5` or
  similar — which is what the synth tool already supports per
  the stage 01 doc.

Bar 2 (three distinct meters) is the only bar that lives or dies
on wiring; that one is ✅.

## Why this is the same problem as stage 01's bar #2

Stage 01's earlier session note already names the root cause:
the durable scheduler claims at 60-second intervals regardless of
the cron expression in the YAML (`*/5 * * * * *` is irrelevant
when claim cadence is 60 s). Stage 02 inherits the same cadence;
no amount of correct stage-02 work changes the row-count rate
the producer can sustain.

The right fix is in `starter-flow-surfaces` / the durable
scheduler — not in stage 02's wiring and not in the success bar.
But until that fix lands:

- The stage-02 success bar as-written cannot turn green via a
  literal reading of the bullets.
- Forcing `DATA_FLOW_SPIKE_PROB` near 1.0 at boot would flip
  bar 3 but still not bar 1.

## What I changed

**No code change.** Investigation + bar-vs-reality reconciliation
only.

## What's left

- [ ] **Decide on scheduler claim cadence.** Either land the
      sub-second claim cadence in the durable scheduler (correct
      long-term fix), or amend the stage docs (01 bar 2, 02 bars
      1+3) to match the 60-s reality. Both stages are blocked on
      this. The earlier stage 01 note proposes editing bar 2 to
      `emitted_total ∈ [10, 18]` for a 60-s cron — the same
      shape edit would land here: bar 1 `count(*) ≥ 10`, bar 3
      `countIf(quality='suspect') > 0 OR DATA_FLOW_SPIKE_PROB
      override documented`. Whichever direction you pick, stage 03
      is blocked behind it because every later stage assumes
      sufficient row volume.
- [ ] **Decide on multi-fire residue.** 58 rows from 32 synth
      emissions means each emit lands ~1.8× in CH. The shape is
      still correct, but L2 / L3 marts will dedupe oddly if this
      isn't addressed. Likely the duplicate-fire root cause from
      the stage-01 note still bites at the engine-link layer,
      now visible at the warehouse instead of the log. If the
      cadence fix above also resolves multi-fire, one fix lands
      both.
- [ ] Once both of the above are resolved, re-run the stage 02
      e2e per [USAGE.md §6](./USAGE.md#6-when-you-finish-or-get-stuck).
      Bars should land naturally; no further code changes
      expected in stage-02-owned files (`rubix-tools/warehouse/`,
      `rubix-tools/clickhouse/ch_client_writer.rs`,
      `rubix-flows/flows/data-flow-producer.yaml`,
      `rubix-agent/migrations/0003_meter_readings_raw/`).
- [ ] PROGRESS.md row 2 stays ⬜ until the above. Add this note
      to PROGRESS.md's "Follow-up notes" table alongside the
      stage-01 multi-fire note.

## References

- Stage doc: [`./02-ingest-l1.md`](./02-ingest-l1.md)
- Prior blocker note (resolved by the user's fixes today):
  [`./02-ingest-l1-blockers-2026-05-26.md`](./02-ingest-l1-blockers-2026-05-26.md)
- Stage-01 cadence root cause:
  [`./2026-05-26-data-flow-01-producer-multi-fire.md`](./2026-05-26-data-flow-01-producer-multi-fire.md)
- Stage-01 deeper root cause (multi-fire seed adapter):
  [`./2026-05-26-data-flow-01-producer-multi-fire-root-cause.md`](./2026-05-26-data-flow-01-producer-multi-fire-root-cause.md)
- L1 migration: [`rubix-agent/migrations/0003_meter_readings_raw/up.sql`](../../../crates/rubix-agent/migrations/0003_meter_readings_raw/up.sql)
- Ingest tool: [`rubix-tools/src/warehouse/ingest.rs`](../../../crates/rubix-tools/src/warehouse/ingest.rs)
- ChClient-backed CH writer (B1 fix): [`rubix-tools/src/clickhouse/ch_client_writer.rs`](../../../crates/rubix-tools/src/clickhouse/ch_client_writer.rs)
- Seed adapter link-aware behaviour (B2 fix): [`rubix-agent/src/boot/mcp/register.rs`](../../../crates/rubix-agent/src/boot/mcp/register.rs)
- Producer flow YAML: [`rubix-flows/flows/data-flow-producer.yaml`](../../../crates/rubix-flows/flows/data-flow-producer.yaml)
- `flow_ops.lint` link-or-default rule: [`rubix-tools/src/flow_ops/lint.rs`](../../../crates/rubix-tools/src/flow_ops/lint.rs)
