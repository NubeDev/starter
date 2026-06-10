# Feature: Flows — Zenoh / MQTT Ingest → Buffer → Postgres

> Verified: **WORKING end-to-end on nexus-rewrite, 2026-06-10** — the Zenoh path
> below was run and landed 200/200 rows. The MQTT path (Path B) is still a
> scaffold.

**What we're testing:** datapump telemetry flows over a broker into a Nexus flow,
buffers with backpressure, lands in Postgres via the postgres sink, and is
queryable through `/api/v1/query`.

Architecture recap ([../reference/ARCHITECTURE.md §2](../reference/ARCHITECTURE.md)):
flows are JSON (`input`/`pipeline`/`output`); each node is a `{"type": "<kind>",
…config}` object (the discriminator is **`type`**, not `kind`); `pipeline` is an
**array** of processor nodes; sink is `postgres` with bind-param inserts.

---

## ✅ Path A — Zenoh (native, verified working)

```
datapump --transport zenoh ──▶ zenoh router :7447 ──▶ flow{ zenoh } ──▶ json_to_arrow ──▶ postgres
```

### Prerequisite: build the backend WITH the zenoh feature

The `zenoh` source is gated behind a cargo feature (OFF by default — a plain
build doesn't register the `zenoh` node kind). Run the backend with it on:

```bash
make dev-be FEATURES=zenoh          # adds --features zenoh (wired in the Makefile)
# or directly:  cd backend && cargo run --bin nexus-api --features zenoh
```

Confirm the source is registered:

```bash
curl -s -b "$JAR" $BASE/api/v1/flows/node-types \
  | python3 -c 'import sys,json; print([n["kind"] for n in json.load(sys.stdin)["node_types"] if n["category"]=="input"])'
# → [... 'zenoh']   ← must include zenoh
```

> Auth uses a **cookie jar + CSRF token**, not a bearer. See
> [../reference/API_CHEATSHEET.md](../reference/API_CHEATSHEET.md) — `$JAR` /
> `$csrf` are set up there.

### 1. Bring up the Zenoh router

```bash
make zenoh-install                  # router on tcp/127.0.0.1:7447, REST :8000
```

### 2. (Optional) The table — let the flow create it

**You do not pre-create the table.** Declare the column types once in the
`json_to_arrow` schema and the postgres sink auto-creates the table from that
schema (`create: true`, the default) with the right PG types — `timestamp` →
`timestamptz`, `float` → `double precision`, etc. The declared schema is the
single source of truth for both parsing and storage. Set `create: false` only if
you want to require a hand-made table.

This is how the flow author controls "how the table is made and stored":
`primary_key` becomes the table PK, and `on_conflict` (`error`/`nothing`/`upsert`)
sets duplicate-key behaviour.

### 3. Create + start the flow (schema-driven, recommended)

`flow-zenoh.json` — declares typed columns; the sink creates `telemetry_typed`
from them with a PK and idempotent inserts:

```json
{
  "name": "zenoh-telemetry-ingest",
  "input":  { "type": "zenoh", "endpoints": ["tcp/127.0.0.1:7447"], "key_expr": "rubix/testing/**", "mode": "client" },
  "pipeline": [ { "type": "json_to_arrow", "schema": { "fields": [
    { "name": "tenant_id", "type": "string" }, { "name": "site_id", "type": "string" },
    { "name": "host_uuid", "type": "string" }, { "name": "point_uuid", "type": "string" },
    { "name": "meter_id", "type": "string", "nullable": false },
    { "name": "kind", "type": "string" }, { "name": "secondary_tag", "type": "string" },
    { "name": "value", "type": "float" }, { "name": "unit", "type": "string" },
    { "name": "timestamp", "type": "timestamp", "nullable": false }
  ] } } ],
  "output": {
    "type": "postgres",
    "uri": "postgres://nexus:nexus@127.0.0.1:4770/nexus",
    "table": "telemetry_typed",
    "create": true,
    "primary_key": ["meter_id", "timestamp"],
    "on_conflict": "nothing"
  },
  "enabled": false
}
```

Declared-schema types (`json_to_arrow`): `string` · `int` · `float` · `bool` ·
`timestamp` (RFC-3339 / timezone-less, stored UTC). The sink maps these to
`text` · `bigint` · `double precision` · `boolean` · `timestamptz`.

> **Inferred-schema shortcut:** omit the `schema` and the sink still works, but
> every column infers from the first batch and the auto-created table will type
> the `timestamp` as `text` (the JSON render is a string). Declare the schema when
> you want real typed columns — which you almost always do for dashboards.

```bash
FLOW=$(curl -s -b "$JAR" -X POST $BASE/api/v1/flows \
  -H content-type:application/json -H "X-CSRF-Token: $csrf" \
  -d @flow-zenoh.json | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

curl -s -b "$JAR" -X POST $BASE/api/v1/flows/$FLOW/start -H "X-CSRF-Token: $csrf" \
  | python3 -c 'import sys,json;d=json.load(sys.stdin);print("running:",d["running"],"err:",d["metrics"]["last_error"])'
# → running: True err: None
```

### 4. Pump data

```bash
cd testing/datapump && cargo build
./target/debug/nexus-datapump \
  --transport zenoh --zenoh-endpoint tcp/127.0.0.1:7447 \
  --path-prefix rubix/testing --path-tenant all \
  --tenant-id nexus \                              # ← MATCH your tenant (RLS! see below)
  --sites 3 --meters-per-kind 4 \
  --interval-ms 100 --count 200 --seed 42
```

### 5. Verify

```bash
# flow metrics — should show batches_in == rows_written, write_errors 0
curl -s -b "$JAR" $BASE/api/v1/flows/$FLOW \
  | python3 -c 'import sys,json;d=json.load(sys.stdin)["metrics"];print(d["batches_in"],d["rows_written"],d["write_errors"],d["last_error"])'

# query back through Nexus — time math works because timestamp is a real timestamptz
curl -s -b "$JAR" -X POST $BASE/api/v1/query -H content-type:application/json \
  -H "X-CSRF-Token: $csrf" -d '{"sql":"SELECT kind, count(*) n, avg(value) avg_val FROM telemetry_typed GROUP BY kind"}'
```

**Observed (2026-06-10, schema-driven sink):** table `telemetry_typed`
**auto-created** with `value double precision`, `timestamp timestamptz`,
`PRIMARY KEY (meter_id, timestamp)`; `batches_in: 200, rows_written: 200,
write_errors: 0`; 100 elec + 100 water; re-pump with `on_conflict:nothing` → still
`write_errors: 0`. ✅ No `text` workaround needed.

---

## Path B — MQTT bridge → http_ingest (scaffold)

```
datapump --transport mqtt ──▶ mosquitto :1883 ──▶ [bridge] ──▶ POST /api/v1/ingest/{flow_id} ──▶ flow{ http_ingest } ──▶ …
```

Flow input node: `{ "type": "http_ingest", "capacity": <N> }` (full channel →
429). The bridge is a small MQTT subscriber that re-POSTs each payload to the
ingest endpoint. **To be written.**

---

## Acceptance criteria

- ✅ Backend built with `zenoh`; `node-types` lists the `zenoh` input.
- ✅ Flow created and `running: true` with `last_error: null` after start.
- ✅ With datapump running: `batches_in` > 0, `rows_written` tracks it,
  `write_errors == 0`.
- ✅ `SELECT count(*)` via `/api/v1/query` matches `--count`.
- ✅ Row fields match the payload (`tenant_id`, `site_id`, `kind`, `value`, …).
- ✅ **Determinism:** fixed `--seed --count` ⇒ same row count + per-kind split.
- ✅ **Schema-driven table:** declared schema auto-creates the table with typed
  columns (`timestamptz`/`double precision`), a `primary_key`, and idempotent
  `on_conflict` inserts. No hand-written DDL.
- ⬜ **Backpressure:** drop `--interval-ms` to 10, raise meters; `channel_depth`
  rises but stays bounded, `write_errors` stays 0. (Not yet exercised.)

---

## Known issues / fixes

### 2026-06-10 — postgres sink can't insert into a `timestamptz` column ✅ FIXED
- **Symptom:** `batches_in` climbed but `rows_written: 0`, `write_errors: 6`,
  `last_error: column "timestamp" is of type timestamp with time zone but
  expression is of type text`.
- **Root cause:** the postgres sink bound every JSON string as PG `text`
  (`nexus-engine/src/sink/pg_insert.rs`); Postgres won't implicitly cast `text` →
  `timestamptz` in a parameterized INSERT. The `json_to_arrow` Arrow→JSON render
  also emits a `Timestamp` as a **timezone-less** string (`2024-…T16:26:40`),
  which `parse_from_rfc3339` (offset-required) rejected.
- **Fix (landed):** the sink now binds **typed by the stream's Arrow schema** —
  a string under a `Timestamp`/`Date` column parses (RFC-3339 *or* timezone-less,
  treated as UTC) and binds as `timestamptz`; ints/floats/bools bind natively.
  The sink also **auto-creates the table** from the Arrow schema with the right PG
  types, an optional `primary_key`, and an `on_conflict` policy. One source of
  truth: the declared schema drives parse, DDL, and binding. Verified 200/200
  into an auto-created `telemetry_typed`. Tests in `pg_insert.rs` +
  `postgres_sink.rs`. The old `text`-column workaround is no longer needed.
- **Note:** stop→start resets the in-process metrics (single-node v1) and drains
  the channel. With `on_conflict:nothing` + a PK, re-pumping is idempotent.

---

## Gotchas

- **zenoh feature off** → flow create accepts the JSON, but `start` fails with an
  unknown-source-type error. Always confirm via `node-types` first.
- **RLS tenant mismatch** is the #1 "rows_written > 0 but query returns 0". This
  run set `--tenant-id nexus` to match the admin tenant. datapump defaults to `*`.
  See [../00_setup/DATAPUMP.md](../00_setup/DATAPUMP.md) +
  [../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md).
- **Node discriminator is `type`** (e.g. `{"type":"zenoh"}`), not `kind`, even
  though `node-types` reports each as `kind`. Easy to get backwards.
- **`pipeline` is a JSON array** of processor nodes, not an object.
