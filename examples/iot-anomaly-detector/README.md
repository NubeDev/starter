# `iot-anomaly-detector` — worked-example port

This example is the canonical demonstration of the worked-example
shape from the Warehouse SCOPE post-review walkthrough. **It does
not contain a single line of ClickHouse SQL.** Every CH read or
write flows through `starter-warehouse` → `starter-store-warehouse`;
the binary is a thin flow driver, not a polling loop.

## What it does

```
MQTT broker ──► tap.write   (raw_events; W7 — never refuses)
                 │
                 ▼
            curate.write   (samples; W6 — refs-as-FK via PG entity lookup)
                 │
                 ▼
            mart_iot_1m   ───► mart.read ──► compute.zscore ──► Verdict
            mart_iot_1h   ───► mart.read ──┘    (warn ≥ 2σ, crit ≥ 3.5σ)
```

### Pipeline steps

1. **Connect** to Postgres (`DATABASE_URL`) and ClickHouse
   (`CLICKHOUSE_URL`).
2. **Apply migrations**:
   * `starter_store_postgres::dimensions::DIMENSIONS_MIGRATION_SOURCE`
     — the W1 catalog (`entities`, `entity_refs`, `marts`,
     `cleaners`, `sandboxes`, `tag_definitions`,
     `tag_prefix_registry`, `ext_manifest_approvals`).
   * `starter_store_warehouse::MigrationRunner` — `raw_events`,
     `samples`, `events`, `documents`, plus the `entities_dict`
     `Dictionary(SOURCE(POSTGRESQL(...)))` that bridges PG → CH
     read-side per W2.
3. **Record the inline `starter-ext-iot` manifest-hash approval**
   in `ext_manifest_approvals` (W12) so the mart definitions that
   follow are accepted as `ext:`-authored rather than quarantined
   on creation.
4. **`mart.define`** two AggregatingMergeTree-backed marts —
   `mart_iot_1m` (60 s buckets) and `mart_iot_1h` (3 600 s
   buckets) — over `samples` grouped by
   `(device_id, location, metric)`.
5. **Subscribe** to `MQTT_TOPIC` (default `iot/+/+`) and, for
   every incoming message:
   * call `WarehouseRuntime::tap_write` (`raw_events`, with a
     `parse_error` tag if the payload is not the expected JSON
     shape — W7 means tag.write **never refuses**);
   * upsert the device entity in Postgres dimensions;
   * call `WarehouseRuntime::curate_write_sample` (`samples`, with
     the entity lookup; W6 — refs are FKs).
6. **Every 30 s** run two `mart.read` calls — one against
   `mart_iot_1h` for the baseline, one against `mart_iot_1m` for
   the recent window — and emit a `Verdict` per anomaly detected
   by an in-process z-score computation. The `mart.read` envelope
   carries the W11 `dimension_freshness` block; downstream
   consumers can short-circuit when the entities dictionary
   refresh has failed.

## Trust gates worth calling out

* **Manifest-hash trust gate (W12).** The constant
  `EXT_MANIFEST_HASH = "iot-ext-v1"` is recorded in
  `ext_manifest_approvals` at boot. Both mart definitions carry
  `ext_manifest_hash: Some("iot-ext-v1")`, which matches the
  approval row. If the hash were bumped without re-approval — for
  example by editing the extension's manifest — the next
  `mart.define` call would *re-quarantine* every previously live
  ext-authored mart in the same Postgres transaction. The catalog
  audit job surfaces this drift on `/api/warehouse/audit`.
* **W11 `dimension_freshness` badge.** Every `mart.read` envelope
  includes the `dimension_freshness` block; the driver logs it
  alongside the row count. A dashboard consumer would render this
  as a per-card freshness badge:
    * `fresh` (refresh in the last 60 s),
    * `stale_within_bound` (between 60 s and 600 s — the
      `entities_dict` `LIFETIME(MIN 300 MAX 600)` window),
    * `stale_beyond_bound` (older than the LIFETIME max),
    * `failed_refresh` (last refresh errored — HTTP 503 from
      `/api/warehouse/status`).
* **W14 filter contract.** The `mart.read` filter is a
  `starter_tags::TagQuery`. Only keys that appear in the catalog
  row's `promoted_columns` may be referenced; everything else
  yields HTTP 400 with a structured body naming both the
  unsupported keys and the promoted set. `mart_iot_*` promote
  `device_id`, `location`, `metric` — a filter referencing
  `floor` is rejected at the read seam (read the SCOPE for the
  full body shape).

## Running against the docker-compose stack

```
# 1. Bring up ClickHouse, Postgres, and an MQTT broker.
docker compose -f docker/docker-compose.clickhouse.yml up -d
docker run -d --name iot-pg -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres postgres:16
docker run -d --name iot-mqtt -p 1883:1883 eclipse-mosquitto:2

# 2. Run the detector.
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
export CLICKHOUSE_URL=http://127.0.0.1:8123
export CLICKHOUSE_DB=demo
export CLICKHOUSE_USER=demo
export CLICKHOUSE_PASSWORD=demo
export WAREHOUSE_PG_HOST=host.docker.internal   # so CH can dial PG
export WAREHOUSE_PG_DB=postgres
export MQTT_HOST=127.0.0.1
export MQTT_TOPIC='iot/+/+'
cargo run -p iot-anomaly-detector
```

Then publish sample readings:

```
mosquitto_pub -h 127.0.0.1 -t iot/dev-a/temp -m '
  {"device_id":"dev-a","location":"lab","metric":"temp",
   "unit":"C","value":21.3}'
```

## "No direct ClickHouse" enforcement

```
cargo tree -p iot-anomaly-detector | grep -i clickhouse
```

should show the `clickhouse` crate reachable **only** under
`starter-store-warehouse` (and transitively `starter-warehouse`).
There is no `clickhouse = "…"` entry on this crate's
`Cargo.toml`; the binary is a pure consumer of the warehouse
capability.

## Why the old polling-loop binary was retired

The previous shape was an `iot_readings` table + a hand-written CH
SQL query that the binary ran every 30 s. That design baked the
mart logic into the binary; bumping the rollup window meant
editing the SQL. The port moves the rollup logic into a *mart
catalog row* (W5) which is generated DDL, hot-swappable, and
auditable. The binary just reads.
