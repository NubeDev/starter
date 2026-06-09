# WS-08 — Datasource Breadth & Connectors

> **Status:** Not started · **Wave:** 2 · **Owner:** _unassigned_
> **Depends on:** nothing hard; feeds WS-06 palette · **Migration:** `0013_datasource_kinds.sql`
> **Read first:** GAP_ANALYSIS §2.8, NEXUS.md §2 (ArkFlow connectors) & §3 (PromQL caveat)

## Goal
Deliver on "any data source." Today **Postgres is the only queryable kind**; ArkFlow speaks far
more upstream but **none are registered**. Wire the connectors the energy/water/HVAC fleet actually
needs — **MQTT/Modbus** (live device data) and time-series stores — each with a config form, a test
path, a secret model, and time-aware querying. Phased: pick the 2–3 the business needs first.

## Current state (evidence)
- `DatasourceKind` has **one** variant: `Postgres` (`datasource/shared.rs`).
- Postgres connector real (`datasource/postgres/**`); query path solid (`query/run.rs`).
- Registered engine inputs: only `http_poll` + `simulator` (`registry/inputs.rs`). Kafka/MQTT/
  Modbus/HTTP/file exist **in ArkFlow upstream** but are **not registered for Nexus**.
- NEXUS.md §3: native **PromQL/LogQL is "product-sized,"** not a freebie — a raw HTTP-proxy input is
  small but real PromQL compat is its own effort. Scope accordingly.

## Scope (phased — do the business-priority subset first)
For **each** connector below: (a) register the ArkFlow input/builder in `registry/inputs.rs` (🔶
shared — append); (b) add a `DatasourceKind` variant (🔶 `datasource/shared.rs` — append) +
per-kind config DTO; (c) a UI **config form** (`features/datasources/forms/<kind>.tsx`); (d) a
**`POST /datasources/test`** path that validates connectivity with the raw config (also closes the
known "test only works after save" gap in `TODO-FOR.UI.md`); (e) **secret handling** via the
existing envelope encryption; (f) a `Dialect`/query-shaping impl so WS-03 time macros work.

Priority order (confirm with business):
1. **MQTT** (input) — subscribe to topics; for *live panels/flows* primarily. Pairs with the SSE
   path; mostly a flow/stream source rather than ad-hoc query.
2. **Modbus** (input) — device registers; the IoT core of the project vision.
3. **HTTP/REST query datasource** — query a JSON/REST API (distinct from `http_poll` ingestion);
   map response → rows for panels.
4. **Time-series store** — pick one of: **Prometheus** (HTTP-proxy input forwarding PromQL — scoped,
   *not* full PromQL parity, per NEXUS.md §3), **InfluxDB**, or lean on **Timescale** (already
   Postgres — add time-series-aware querying/continuous-aggregate awareness).
5. **Kafka** (input) — streaming source for flows/live.

## Design notes
- **The registry is the extension point, not the enum** (NEXUS.md §5.4): adding a connector =
  implement/register `Input` + `InputBuilder`; the `DatasourceKind` variant is the product-facing
  label. Keep both additive.
- **Query vs stream sources differ.** Postgres/HTTP-REST/Prometheus are *query* (request→rows);
  MQTT/Modbus/Kafka are *stream* (live panels/flows). Be explicit per kind which surface it serves —
  some only make sense as flow/live sources, not ad-hoc `POST /query`.
- **Time-series shaping**: WS-03's `Dialect` trait is where each kind expresses `$__timeGroup` /
  `$__timeFilter`. A non-SQL source (Prometheus) needs its own macro→native-query mapping.
- **Secrets**: every credentialed kind uses the existing envelope-encryption + redacted-GET +
  decrypt-at-stream-build boundary (NEXUS.md §4). No plaintext, no return over API.
- **Build weight** (NEXUS.md Risk #4): some ArkFlow connectors pull heavy deps (protoc/PyO3). Gate
  features so we don't bloat the binary for unused connectors.
- **PromQL/LogQL**: ship the *scoped* HTTP-proxy form (forward a query string), and **clearly label
  it** "basic PromQL passthrough, not full compatibility." Full parity is a separate future effort.

## Acceptance criteria
- [ ] At least the top-2 priority connectors registered, configurable, testable, secret-protected.
- [ ] `POST /datasources/test` validates raw config pre-save (closes the TODO-FOR.UI gap).
- [ ] A live panel/flow ingests from MQTT or Modbus end-to-end.
- [ ] Per-kind `Dialect` makes WS-03 time macros work for each *query* connector.
- [ ] Credentials encrypted at rest, redacted on GET, decrypted only at stream-build.
- [ ] Build stays lean (unused connector features gated off).
- [ ] Tests: per-connector config validation, test-path, secret redaction, dialect macro output.

## Out of scope (hand off)
- The flows palette that *shows* these → **WS-06** (coordinate so connectors + palette land together).
- Full PromQL/LogQL compatibility → explicitly a separate future effort (NEXUS.md §3).
- Heavy ETL/CDC/lake replication → out of v1 (NEXUS.md §13).
