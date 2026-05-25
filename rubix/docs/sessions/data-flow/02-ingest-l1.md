# Stage 02 — raw landing in ClickHouse (L1)

## Scope

**In:** an L1 table `rubix.meter_readings_raw` in the `rubix`
ClickHouse database, defined via `rubix.clickhouse.rule.write`, and
a write path that lands every row stage 01's producer emits — bit
for bit, no cleaning. Tenancy is per-row.

**Out:** any normalisation, gap-fill, downsampling, mart creation,
or alerting. Stage 03 owns L2.

## L1 schema (locked)

```sql
CREATE TABLE rubix.meter_readings_raw (
  tenant_id  String,
  meter_id   String,
  kind       LowCardinality(String),    -- 'electricity' | 'water'
  unit       LowCardinality(String),    -- 'kWh' | 'L'
  epoch_ms   Int64,
  value      Float64,                   -- NaN allowed for elec.hvac mess
  quality    LowCardinality(String)     -- 'ok' | 'suspect'
) ENGINE = MergeTree
PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000))
ORDER BY (tenant_id, meter_id, epoch_ms);
```

Matches the canonical layout in
[design/warehouse/README.md §"The layers"](../../design/warehouse/README.md#the-layers)
(L1 raw, partitioned monthly, ordered by tenant + entity + time).
Database is `rubix` per the warehouse routing note in that same
doc — no unqualified writes.

Retention: 14 days. Set via `rubix.clickhouse.retention.set`
after the rule lands. Rationale: the L2 mart in stage 03 holds
months; L1 only needs to outlive the replay window for late-arriving
fixes. Hard rule from the warehouse design doc:
**L1 retention < L2 retention** — never the other way.

## Write path

Pick **one** path and write the choice into "Decisions taken" at
the bottom before starting:

- **A. Bind a `rubix.warehouse.ingest` tool** that takes a batch of
  rows matching stage 01's wire shape and writes them through
  `starter-store-clickhouse::ChClient::insert`. This is the seam
  the producer flow already targets. **Preferred.**
- **B. Reuse `rubix.clickhouse.rule.write` for both DDL and
  inserts.** Not recommended — `rule.write` is a DDL verb, not a
  bulk writer. Only pick this if A's binding work is blocked.

Path A's binding lands in `rubix-tools/src/warehouse/ingest.rs`,
mirroring the existing `system::disk` write path that already uses
`ChClient` against `rubix.system_disk_history`. ≤ 150 LOC of verb
+ DTO + descriptor. No undo wiring (ingest is append-only; undo
of a single row makes no sense — operators undo at the mart
layer).

## Pre-flight

- Stage 01 success bar green — the producer runs and at least the
  emitted rows are visible in the agent log.
- ClickHouse reachable: `curl -s http://127.0.0.1:8124/ping`
  returns `Ok.`
- The agent is talking to the `rubix` database, not `default`.
  Verify by querying the migration runner's startup log: it must
  emit `CREATE DATABASE IF NOT EXISTS rubix` exactly once on boot
  (see warehouse design doc §"Database routing").

## Steps

1. Land the L1 DDL through the rule verb (a single rule body
   carrying the `CREATE TABLE rubix.meter_readings_raw …` from
   above). The verb captures the prior DDL (none, first time) and
   records a Change so `rubix.undo.last` cleanly drops the table —
   see [design/clickhouse-rules/README.md §"kind = clickhouse_rule"](../../design/clickhouse-rules/README.md#kind--clickhouse_rule).

   ```bash
   curl -s -b /tmp/smoke-cookies.txt -X POST \
     http://127.0.0.1:8088/api/v1/tools/rubix.clickhouse.rule.write \
     -H 'content-type: application/json' \
     -d '{ "rule_name": "meter_readings_raw",
           "body": "CREATE TABLE rubix.meter_readings_raw ( ... ) ENGINE = MergeTree PARTITION BY toYYYYMM(toDateTime(epoch_ms / 1000)) ORDER BY (tenant_id, meter_id, epoch_ms);" }'
   ```

2. Set the 14-day TTL:

   ```bash
   curl -s -b /tmp/smoke-cookies.txt -X POST \
     http://127.0.0.1:8088/api/v1/tools/rubix.clickhouse.retention.set \
     -H 'content-type: application/json' \
     -d '{ "table_name": "meter_readings_raw", "days": 14 }'
   ```

3. (Path A) Bind `rubix.warehouse.ingest` in `rubix-tools` and
   wire it into the tool registry. The producer flow's `ingest`
   tool node now lands rows.

4. Let the producer flow run for 5 minutes.

5. Verify rows land:

   ```bash
   curl -s -X POST -d \
     "SELECT meter_id, count(), countIf(quality='suspect')
        FROM rubix.meter_readings_raw
       WHERE epoch_ms >= (toUnixTimestamp(now()) - 300) * 1000
       GROUP BY meter_id FORMAT TSV" \
     http://127.0.0.1:8124/
   ```

## Success bar

Stage 02 is done when **all three** are true:

1. `SELECT count() FROM rubix.meter_readings_raw` is ≥ 200 after a
   5-minute producer run.
2. Three distinct `meter_id` values appear, matching the three
   meters in [README.md](./README.md).
3. `countIf(quality='suspect')` is **non-zero** (the spike rows
   landed un-cleaned — proves the writer is faithful, not
   accidentally filtering).

## If it fails

In order, check:

1. **Database routing** — `SELECT count() FROM
   default.meter_readings_raw` returns > 0. Means the agent's
   `ChClient` is bound to `default`, not `rubix`. Fix in
   `rubix-agent::boot::clickhouse`; this is the bug the warehouse
   design doc explicitly calls out.
2. **Schema mismatch** — `ChClient::insert` returns
   `unknown column` / `cannot parse`. The wire shape diverged
   from the DDL. Re-pin the wire shape against
   [01-producer.md §"Wire shape"](./01-producer.md#wire-shape-locked-both-a-and-b-emit-this).
3. **Producer emits but ingest tool not found** — the new
   `rubix.warehouse.ingest` is missing from `tools/list`. The
   tool registry didn't pick it up; check the boot composer
   wiring (same shape as `rubix.system.disk`).

Write follow-up notes as `YYYY-MM-DD-data-flow-02-ingest-<topic>.md`
and stop. Do not start stage 03 until the success bar is green.

## Decisions taken

- [ ] Path A (bind `rubix.warehouse.ingest`)  /  [ ] Path B
- Table name: `rubix.meter_readings_raw` (do not rename — stage 03 reads this)
- L1 retention: 14 days
