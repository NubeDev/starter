# datapump — The Test Data Generator

> Verified: nexus-rewrite tip on 2026-06-10. Source: `testing/datapump/`.
> datapump now pumps **MQTT or Zenoh** via `--transport`. Both transports are
> live.

`datapump` publishes synthetic Rubix-style meter telemetry: a fleet of electric
and water meters across N sites, deterministic per seed, at a fixed interval. It
is the single source of "good data" for the testing suite.

---

## Run it

```bash
cd testing/datapump

# MQTT (needs a bridge to reach Nexus — see FLOWS_MQTT_INGEST.md)
cargo run -- --transport mqtt --host 127.0.0.1 --port 1883

# Zenoh (native zenoh source — no bridge)
cargo run -- --transport zenoh --zenoh-endpoint tcp/127.0.0.1:7447
```

`--transport` is `mqtt` (default) or `zenoh` (the misspelling `zenho` is accepted
as an alias); env `DATAPUMP_TRANSPORT`. Defaults: 3 sites, 4 meters/kind, 1s
interval, infinite count.

Full knobs:

```bash
cargo run -- \
  --transport zenoh \                 # DATAPUMP_TRANSPORT (mqtt|zenoh|zenho)
  --zenoh-endpoint tcp/127.0.0.1:7447 \ # ZENOH_ENDPOINT (zenoh only)
  --host 127.0.0.1 --port 1883 \      # MQTT_HOST / MQTT_PORT (mqtt only)
  --client-id nexus-mqtt-data-gen \   # MQTT_CLIENT_ID (mqtt only)
  --username u --password p \         # MQTT_USERNAME / MQTT_PASSWORD (mqtt, optional)
  --qos 0 \                           # MQTT_QOS (mqtt only, 0|1|2)
  --path-prefix rubix/testing \       # DATAPUMP_PATH_PREFIX (alias --topic-prefix)
  --tenant-id '*' \                   # RUBIX_TENANT_ID  (written INTO payload)
  --path-tenant all \                 # RUBIX_TOPIC_TENANT (topic/key segment; '*' unsafe in topics)
  --sites 3 \                         # RUBIX_SITE_COUNT
  --meters-per-kind 4 \               # RUBIX_METERS_PER_KIND (elec + water each → 8 meters/site)
  --interval-ms 1000 \                # RUBIX_INTERVAL_MS
  --count 200 \                       # RUBIX_COUNT (omit → run until Ctrl-C)
  --seed 42                           # RUBIX_SEED (determinism)
```

Every flag has an env-var equivalent (shown above), so you can also drive it from
a `.env` or a scenario script. `--path-prefix` keeps `--topic-prefix` as an alias
for back-compat.

---

## Payload (one JSON object per publish)

From `src/model.rs` `Telemetry`:

```json
{
  "tenant_id": "*",
  "site_id": "site-001",
  "host_uuid": "host-001",
  "point_uuid": "point-001",
  "meter_id": "meter-001",
  "kind": "elec",
  "secondary_tag": "power",
  "value": 42.5,
  "unit": "kWh",
  "timestamp": "2026-06-10T15:23:45Z"
}
```

| Field | Meaning |
|-------|---------|
| `tenant_id` | tenant written into the row. Default `*` (Rubix fixture convention) — **may not match your Nexus tenant UUID**; see RLS note below. |
| `kind` | `elec` or `water` |
| `secondary_tag` | `power` (elec) / `reading` (water) |
| `unit` | `kWh` (elec) / `m3` (water) |
| `value` | random walk around a per-meter base, deterministic per `--seed` |
| `*_uuid`, `meter_id`, `site_id` | stable identifiers across a run |

---

## Topic / key structure

```
{prefix}/{path_tenant}/{site_id}/{kind}/{meter_id}
```

Example: `rubix/testing/all/site-001/elec/meter-001`. The same string is used as
the MQTT topic and (planned) the Zenoh key expression, so a flow's `key_expr`
like `rubix/testing/**` subscribes to the whole fleet.

---

## Determinism

Same `--seed` + same fleet size (`--sites`, `--meters-per-kind`) ⇒ same meter ids
and the same value sequence. Use this for regression scenarios: a fixed seed
gives a reproducible dataset you can assert exact aggregates against.

---

## Getting the stream into Nexus

datapump publishes to a broker; it does **not** talk to the Nexus API directly.
Two paths (full setup in
[../features/FLOWS_MQTT_INGEST.md](../features/FLOWS_MQTT_INGEST.md)):

1. **`--transport zenoh` → `zenoh` source** (native, no bridge). Requires the
   backend built with the `zenoh` feature and a flow whose source `key_expr`
   matches the path prefix (e.g. `rubix/testing/**`). Cleanest path. ⭐
2. **`--transport mqtt` → bridge → `http_ingest`** — a small subscriber that
   POSTs payloads to `POST /api/v1/ingest/{flow_id}`. Zero backend feature flags.

---

## RLS gotcha (read this before debugging "no rows")

datapump defaults `tenant_id` to `*`. Nexus rows are RLS-scoped to a real tenant
UUID. If your flow writes the payload's `tenant_id` verbatim, rows land under
tenant `*` and are invisible to a token scoped to your admin tenant. Either:

- set `--tenant-id <your-tenant-uuid>` so payloads carry the right tenant, **or**
- have the flow/sink stamp the tenant from the request context.

See [../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md) → "Data written but
invisible".

---

## Source layout (for when you fix/extend the generator)

| File | Role |
|------|------|
| `src/cli.rs` | flags / env vars (`Args`) |
| `src/config.rs` | `TransportKind` (mqtt/zenoh/zenho) + broker config |
| `src/transport.rs` | transport dispatch |
| `src/mqtt.rs` | MQTT publisher (rumqttc) |
| `src/zenoh.rs` | Zenoh publisher |
| `src/generator.rs` | the synthetic fleet + value walk |
| `src/model.rs` | `Telemetry` payload + `path()` topic/key builder |
| `src/run.rs` | the publish loop |

> Note: `Cargo.toml` may still name the bin `nexus-mqtt-data-gen` even though the
> crate is now "datapump". Confirm the bin name if `cargo run` complains, and fix
> this doc's commands if it changed.
