# RW-08 — Backpressure hardening, flow metrics, soak test

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-04. Last in queue — it validates the whole ingest path.

## Why

The design claim is: 1000s of devices → broker QoS (L1) → bounded channels (L2) →
batched writes, with no unbounded memory growth and no silent data loss. This WS proves
it and makes the behavior observable.

## Scope

1. Metrics on the ingest path (extend `FlowMetrics` in `flow/manager.rs` — WS-06 added
   running/last_started_at/last_error): per-flow `batches_in`, `rows_written`,
   `channel_depth` (sampled), `flush_count`, `write_errors`, `last_write_ms`. Surface on
   the existing flow list/detail DTOs (DTO-first, openapi + codegen; the flows UI table
   can show them — UI wiring optional, follow-up if large).
2. Failure semantics, implemented + documented in `rewrite/BACKPRESSURE.md`:
   - sink write error → bounded retry with backoff (cap N attempts), then flow enters
     `last_error` state WITHOUT dropping the in-flight batch silently — per-flow
     `on_error: halt|drop|dlq` (default halt); `dlq` routes failed batches to an RW-04
     file writer (Parquet dead-letter path) — halt-vs-silent-drop alone is a brutal
     binary for a device fleet;
   - source read error → `source_on_error: retry_backoff (default, capped) | halt` per
     roadmap §6; verify the MQTT source's internal reconnect (rumqttc) composes with
     rather than masks this policy;
   - cancellation mid-batch → flush partial batch before close (verify RW-04 contract);
   - channel-full source behavior → source `read()` naturally blocks on send; document
     that broker-side queueing (MQTT QoS) is the upstream buffer.
3. Soak test (`backend/tests/soak/`, `#[ignore]`-by-default + a `make soak` target —
   ignored tests as an explicit opt-in run is the sanctioned pattern here, NOT a dodge):
   simulator source at high rate (e.g. 50k rows/min) → datasource sink → docker Postgres
   for ≥10 min: assert bounded process RSS (read /proc/self), zero lost rows
   (count in == count landed), p99 flush latency recorded; kill the DB for 30s mid-run →
   flow surfaces the error, resumes per `on_error` policy, totals reconcile. Include a
   FAT-BATCH case: a source/processor emitting single multi-million-row batches must get
   sliced by the §6 `max_batch_rows` bound and stay within the RSS assertion — bounded
   channel depth alone proves nothing if batches are unbounded.
4. Load-shed guard on LiveRunner SSE broadcast (slow client = lagged receiver): verify
   the existing `tokio::broadcast` lag behavior drops for THAT subscriber only and the
   seq-number resume contract reports the gap; test it.

## Acceptance

- Soak target runs green locally (document runtime + how to invoke in BACKPRESSURE.md).
- Metrics visible on flow detail route (e2e asserts fields present + monotonic).
- DB-outage chaos case: no panic, no silent loss under `halt`, documented loss counter
  under `drop`.
- `BACKPRESSURE.md` explains L1/L2/batching + failure semantics in one page.
