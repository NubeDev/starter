# RW-09 — Transport breadth: http_ingest + Zenoh source

> Verified: 2026-06-10 against nexus-rewrite. §0: re-grep cites before coding.
> Depends on RW-04 (datasource sinks) + RW-07 (`ingest.write` backpressure semantics —
> http_ingest is its REST twin). Last in queue after RW-08.

## Why

Incoming data must not be limited to MQTT (human requirement 2026-06-10). The ingest
column is: pull (http_poll, file), subscribe (mqtt, zenoh), push (http_ingest,
extension `ingest.write`) — all converging on the same bounded-channel → batched-sink
path. This WS adds the two missing first-class transports.

## Scope

1. `source/http_ingest.rs` + a thin route — push ingestion over plain HTTP:
   - a flow with `{ "input": { "type": "http_ingest" } }` exposes
     `POST /api/v1/ingest/:flow_id` (token-authed, tenant-scoped via the principal —
     the flow must belong to the caller's tenant; 404 not 403 on cross-tenant probe,
     matching existing route behavior).
   - body: JSON object or array of objects → json_to_arrow (schema-stability contract
     §6 applies) → the flow's bounded channel.
   - backpressure: channel full → `429` + `Retry-After` seconds — the SAME contract as
     RW-07's `ingest.write` host method; share the implementation, don't duplicate it.
   - DTO-first for the response shape; mirrored route tests incl. the 429 path
     (tiny channel capacity) and cross-tenant denial.
2. `source/zenoh.rs` — Eclipse Zenoh subscriber source (https://zenoh.io), feature-gated
   `zenoh` OFF by default (WS-08b rumqttc precedent; roadmap §8 amended):
   - config: `{ "type": "zenoh", "endpoints": [...], "key_expr": "site/**",
     "mode": "client|peer" }` + payload decode (json default; raw-bytes passthrough as
     a `binary` column is acceptable v1 for non-JSON payloads).
   - subscriber → batch accumulation (respect `max_batch_rows`) → channel; implement
     `Source::commit()` as no-op v1 (zenoh subscriber has no ack; document at-most-once)
     — if zenoh's reliability/queryable features make at-least-once cheap, note it as a
     follow-up, don't build it now.
   - datasource-kind manifest entry (`zenoh` stream kind, WS-08b declarative format) so
     the datasource UI can offer it with a probe (open session + scout, bounded timeout).
   - cancellation: token → undeclare subscriber + close session cleanly (test).
   - unit tests behind the feature; one docker/process-gated e2e if a zenoh router is
     practical in CI, otherwise a loopback peer-mode test in-process (zenoh supports
     in-process peers — no external broker needed for tests, which is a major win).
3. Docs: extend rewrite/BACKPRESSURE.md (RW-08) with the push-path semantics
   (429/Retry-After) and the zenoh at-most-once note.

## Non-goals

Zenoh queryables/storages integration (powerful — edge-side query — but a separate
follow-up WS once subscription ingest is proven), zenoh-bridge-mqtt deployment guidance
(ops doc, not code), Kafka/NATS/Modbus (extensions via RW-07 when needed).

## Acceptance

- `curl -X POST /api/v1/ingest/:flow_id` with JSON lands rows in a datasource sink e2e;
  channel-full returns 429 + Retry-After; cross-tenant probe 404s.
- Feature-gated zenoh build: `cargo test --features zenoh` green incl. loopback
  pub→source→sink round-trip; default build contains zero zenoh deps (`cargo tree` proof
  in session log).
- Flow configs for both sources validate + round-trip through the flows API/UI builder
  node-types listing (engine `describe()` includes them; zenoh only when compiled in).
- cargo + UI gates green; openapi/codegen committed; pushed to origin nexus-rewrite.
