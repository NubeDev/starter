# Triage — Symptom → Root Cause

> After [CAPTURE.md](CAPTURE.md). Find your symptom, run the confirming check,
> then go to [FIX_LOOP.md](FIX_LOOP.md). Don't skip the confirming check — the
> obvious cause is often not the real one.

---

## Symptom index

| Symptom | Most likely cause | Confirm with | Section |
|---------|-------------------|--------------|---------|
| Flow `rows_written` > 0 but query returns 0 | RLS tenant mismatch | `pg_state.txt` with correct vs `*` tenant | [§1](#1-data-written-but-invisible) |
| `rows_written` stays 0, `batches_in` 0 | source not receiving (broker/key_expr/feature) | `flows.json` + broker sub | [§2](#2-no-data-arriving) |
| `batches_in` > 0 but `rows_written` 0, `write_errors` > 0 | sink/schema/insert error | `backend.log` | [§3](#3-arriving-but-not-landing) |
| `channel_depth` grows unbounded | backpressure / slow sink | `flows.json` over time | [§4](#4-buffer-not-draining) |
| API call 401/403 | auth/tenant/grant | `request.txt`, `/api/v1/me` | [§5](#5-auth--access) |
| API call 4xx with body schema error | contract drift | `openapi_slice.json` | [§6](#6-contract-drift) |
| Variable value breaks query / returns wrong rows | injection vs bind, resolution order | panel SQL + var config | [§7](#7-variables--context) |
| Alert never fires | evaluator not running / threshold | `backend.log`, rule config | [§8](#8-alerts-not-firing) |
| Insight preview errors | Compile/LimitExceeded/Runtime | error class in response | [§9](#9-insight-errors) |

---

## 1. Data written but invisible

The classic. The flow wrote rows under one tenant; your read uses another. RLS
hides the rest.

- Confirm: in `pg_state.txt`, compare `SET app.tenant_id = '<your-tenant>'` vs
  `SET app.tenant_id = '*'`. If the `*` count is high and yours is 0 → mismatch.
- Cause: datapump defaults payload `tenant_id` to `*`; your token is a real UUID.
- Fix paths: run datapump with `--tenant-id <your-tenant-uuid>`, **or** make the
  sink stamp tenant from context. See [DATAPUMP.md](../00_setup/DATAPUMP.md).

## 2. No data arriving

`batches_in == 0`. The source isn't getting samples.

- Broker reachable? `nc -z` the port; for MQTT `mosquitto_sub -t '#' -v` should
  show datapump's messages.
- Zenoh path: is the backend built with the `zenoh` feature? Does the flow
  `key_expr` match datapump's `--path-prefix` (e.g. `rubix/testing/**`)?
- MQTT path: is the bridge running and POSTing to the **right `flow_id`**?
- Is the flow actually **started/enabled** (not just created)? Check `flows.json`.

## 3. Arriving but not landing

`batches_in` rising, `write_errors` > 0.

- `backend.log` will name it: schema mismatch (`json_to_arrow` couldn't shape a
  field), missing/extra column vs the target table, type coercion, or a
  constraint. Fix the flow `pipeline`/`output` config or the table.

## 4. Buffer not draining

`channel_depth` climbs without bound under load.

- Sink is slower than the source. Expected briefly under burst; a problem if it
  never recovers. Check `write_errors`, DB connection pool saturation, and
  whether inserts are batched. This is the backpressure test from
  [FLOWS_MQTT_INGEST.md](../features/FLOWS_MQTT_INGEST.md).

## 5. Auth / access

- 401 → token missing/expired. Re-login. `/api/v1/me` should return your user.
- 403 → tenant or grant. As a non-admin, you need an explicit grant on the
  `nexus.nav_node` / `nexus.dashboard`. As admin you shouldn't (default_policy).
  Cross-tenant access is denied by design.

## 6. Contract drift

A request 4xx's on body shape, or a documented path is gone.

- `grep "<path>" backend/openapi.json`. The doc is stale or the request is wrong.
  Fix the cheatsheet/feature doc, bump its `Verified:` line. If the path is
  genuinely gone, that's a real backend change — check recent commits / `WS-xx`.

## 7. Variables / context

- Wrong rows: check resolution order (constants → custom/datasource/interval →
  query → built-ins) and whether the value actually bound. A value containing a
  quote that **breaks** the SQL means it was inlined, not bound — a real bug
  (WS-03 contract says bind as `$N`).
- Context not flowing: nav node `context.values` → `context` variable kind is
  read-only; confirm the node carries the value and the variable reads the right
  source (nav/url/tag/values).

## 8. Alerts not firing

- First: **is anything evaluating rules?** The evaluator scheduler was unverified
  in the 2026-06-10 sweep. grep alert routes / `nexus-insights` for the scheduler.
  No evaluator → no fire, regardless of config.
- Then: did the data actually cross the threshold? Did `for_secs` dwell elapse?
  Is there a silence active? Is the channel wired?

## 9. Insight errors

- The response error class tells you where: `Compile` (script syntax / unknown
  primitive — check `GET /insights/functions`), `LimitExceeded` (result grew past
  the cap — your transform fans out rows), `Runtime` (bad op on the frame).
