# data-flow — end-to-end pipeline proof (energy + water, messy)

Scenario-scoped working notes for proving the rubix stack end-to-end
on **deliberately messy energy + water meter data**: irregular
sample cadence, missing intervals, value spikes (sensor glitches),
and stuck-at-zero stretches.

The goal of these notes is **not** to fix everything in one pass.
Each stage doc below is small, runnable on its own, and ends with a
clear success bar. Pick a stage, prove it, fix what's broken,
write a follow-up session note if anything spills, then stop.

---

## The scenario

A small "site" has 3 meters:

| Meter id            | Kind        | Cadence (nominal) | Mess we inject                                |
|---------------------|-------------|-------------------|-----------------------------------------------|
| `site-a.elec.main`  | Electricity | every 60 s        | gaps (10–30 min drop-outs), spikes (×50)      |
| `site-a.water.main` | Water       | every 5 min       | stuck-at-zero stretches (sensor freeze)       |
| `site-a.elec.hvac`  | Electricity | every 60 s        | timestamp drift (±20 s jitter), occasional NaN|

Units: `kWh` (cumulative for electricity), `L` (cumulative for
water). Tenancy: a single `site-a` tenant for v0.

Why these shapes: they are the four failure modes that break naive
dashboards (gaps → broken interpolation, spikes → axis blow-out,
stuck zeros → bad averages, jitter → wrong bucket alignment). If
the stack handles these, it handles real meters.

---

## Stack map (what each stage exercises)

```
┌──────────────────────────┐
│ 01-producer              │  flow OR extension that emits readings
│ (synthetic, intentionally│  via rubix.warehouse.ingest (or direct
│  messy)                  │  WarehouseClient write — TBD per stage 01)
└────────────┬─────────────┘
             ▼
┌──────────────────────────┐
│ 02-ingest-l1             │  rubix.warehouse.rule.write defines the
│ raw landing table        │  L1 table; producer rows land here as-is
└────────────┬─────────────┘
             ▼
┌──────────────────────────┐
│ 03-clean-to-l2           │  rubix.warehouse.mart.create builds an
│ normalise, gap-fill,     │  L2 curated mart (continuous aggregate);
│ clip spikes, bucketise   │  refresh policy materialises L1 → L2
└────────────┬─────────────┘
             ▼
┌──────────────────────────┐
│ 04-anomaly-rules         │  insights gate / flow rule that watches
│ flag spikes + stuck zeros│  L2 and fires rubix.alert.send
└────────────┬─────────────┘
             ▼
┌──────────────────────────┐
│ 05-dashboard-at-scale    │  SDUI dashboard reading L3 marts; proves
│ render N days of data    │  pagination / downsampling under load
└──────────────────────────┘
```

Design anchors (read once at the start of any stage):

- Overview: [rubix/docs/design/overview/README.md](../../design/overview/README.md)
- Warehouse layers + tenancy: [rubix/docs/design/warehouse/README.md](../../design/warehouse/README.md)
- Warehouse rules / marts / retention verbs: [rubix/docs/design/clickhouse-rules/README.md](../../design/clickhouse-rules/README.md) (legacy directory name; content is `rubix.warehouse.*` since the engine swap — see ADR-004)
- Flow programmer verbs: [rubix/docs/design/flow-programmer/README.md](../../design/flow-programmer/README.md)
- Extension bundle layout: [rubix/docs/design/extensions/README.md](../../design/extensions/README.md)
- Insights / alert dispatch shape: [rubix/docs/design/insights/README.md](../../design/insights/README.md)
- Dashboard / SDUI verbs: [rubix/docs/design/sdui/tools/README.md](../../design/sdui/tools/README.md)
- Reports (later): [rubix/docs/design/reports/README.md](../../design/reports/README.md)
- Tags (placeholder): [rubix/docs/design/tags/README.md](../../design/tags/README.md)

---

## Start here (new AI session, cold repo)

1. Read [USAGE.md](./USAGE.md) — how to bring the stack up, log
   in, and call a verb.
2. Read [PROGRESS.md](./PROGRESS.md) — find the next ⏳ stage.
3. Open that stage's doc (list below) and work it.
4. When the stage's "Success bar" is green, update
   [PROGRESS.md](./PROGRESS.md) and stop. Use
   [_SESSION-TEMPLATE.md](./_SESSION-TEMPLATE.md) for any
   spillover note.

## Stage docs

Work them in order; each one assumes the previous stage's
"Success bar" is met. If it isn't, stop and fix that stage first.

1. [01-producer.md](./01-producer.md) — messy energy + water producer (flow OR extension)
2. [02-ingest-l1.md](./02-ingest-l1.md) — raw landing in the warehouse (TimescaleDB L1)
3. [03-clean-to-l2.md](./03-clean-to-l2.md) — normalise, gap-fill, clip
4. [04-anomaly-rules.md](./04-anomaly-rules.md) — spike + stuck-zero rules
5. [05-dashboard-at-scale.md](./05-dashboard-at-scale.md) — large dataset in the dashboard

---

## How to use these docs (for the next AI session)

Each stage doc has the same five sections so they are scannable:

- **Scope** — exactly what is in/out for this stage.
- **Pre-flight** — what must already be true (services up, prior
  stage's success bar met, fixtures present).
- **Steps** — the minimal command + verb sequence to drive the
  stage. Copy-pasteable.
- **Success bar** — the *one* observable that means this stage
  works. If this is green, the stage is done — do not gold-plate.
- **If it fails** — the first three things to check, in order, and
  a pointer to where to file a follow-up session note. **Do not
  expand scope; write the note and stop.**

Follow-up session notes go next to this README using the existing
naming convention (`YYYY-MM-DD-data-flow-<stage>-<topic>.md`) in
[../](../).

---

## Out of scope for this folder

- Hardening multi-tenancy (single `site-a` tenant only).
- Production blob retention, backups, GDPR export.
- Real meter integration (MQTT / Modbus / BACnet adapters).
- Frontend polish beyond "the chart renders without freezing".
- Wiring this scenario into the weekly `rubix.analytics.report`
  path — that lands after stage 05's success bar is green.
