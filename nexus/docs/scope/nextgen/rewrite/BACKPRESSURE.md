# Backpressure, flow metrics, and failure semantics

> RW-08. One page on how the ingest path stays bounded under load, what it does
> when a node fails, and how to observe and soak-test it.

## The two-level bound (L1/L2) + batching

```
devices → MQTT broker (QoS = L1)  → Source.read()
        → bounded mpsc channel (L2, default 64) → processors → Sink.write()
        → datasource sink (batched: N rows OR T ms) → store
```

- **L1 — broker QoS.** The upstream buffer is the broker. A QoS-capable source
  (MQTT, future queues) does not acknowledge a message until the sink has written
  it (`Source::commit`, called by the pipeline after each successful sink write),
  so unflushed work backs up in the broker, not in process memory.
- **L2 — bounded channel.** The source task pushes batches into a bounded
  `tokio::mpsc` channel. When the sink is slower than the source, the channel
  fills and `Source.read()` naturally blocks on send — backpressure with no
  unbounded queue. There is no load-shed on the ingest path: a full channel
  slows the source, it never drops.
- **Batch-size bound.** The channel bounds *batch count*, not bytes; one fat batch
  would defeat L2. The pipeline slices any batch wider than `max_batch_rows`
  (config, default 8192) — zero-copy via `RecordBatch::slice` — at the source and
  processor-output boundary, so channel depth × `max_batch_rows` bounds in-flight
  rows. The soak's fat-batch case proves a single multi-million-row emit stays
  within the RSS bound.
- **Write batching.** The datasource sink flushes when it has buffered
  `batch_rows` rows OR `batch_ms` has elapsed, whichever first; `close`
  (clean end or cancellation) always flushes the tail, so no row is stranded.

## Per-flow metrics

Each run carries a `FlowMetrics` handle (`nexus-engine/src/flow/metrics.rs`) that
its metered source/sink wrappers bump as batches flow. The flows list/detail
endpoints surface a snapshot on `FlowMetrics` (DTO): `batches_in`, `rows_written`,
`channel_depth` (approximate in-flight gauge), `flush_count`, `write_errors`,
`last_write_ms`, alongside the existing `running` / `last_started_at` /
`last_error`. They are process-local and reset when a fresh run starts.

## Failure semantics

Both policies are additive keys on the existing opaque node config — an older flow
that sets neither gets the safe defaults. A shared capped exponential backoff
(`base × 2^(attempt-1)`, capped at 10s, default 5 attempts) governs both, tunable
per node via a `retry: { max_attempts, base_backoff_ms }` block.

### Sink write error — `output.on_error` (default `halt`)

The sink wrapper retries the write with backoff, counting every failed attempt.
Once retries are exhausted:

- **`halt`** — surface the error; the pipeline stops and the flow records
  `last_error`. The in-flight batch is **not** silently dropped. (Default —
  halt-vs-silent-drop is a brutal binary for a device fleet, so the non-lossy
  side is the default.)
- **`drop`** — discard the failed batch and keep running. Lossy by choice; the
  loss is observable via `write_errors`, never silent.
- **`dlq`** — write the failed batch to a dead-letter Parquet dataset (the RW-04
  `file` writer) under `output.dlq.dir` (default `./dead-letter`), then keep
  running. No silent loss: the rows land on disk. A broken dead-letter path itself
  halts the run rather than dropping.

### Source read error — `input.source_on_error` (default `retry_backoff`)

- **`retry_backoff`** — retry with capped backoff, then surface the error (the run
  halts) if still failing. Never an infinite retry loop. This composes with — does
  not mask — the MQTT source's own `rumqttc` reconnect: rumqttc reconnects the
  transport transparently, and only a read that still errors after that reaches
  this policy.
- **`halt`** — stop on the first read error.

### Cancellation mid-batch

`token.cancelled()` stops the source at its next `.await`; the batches already in
the channel are drained and written, then the sink is closed once (final flush).
No partial batch is lost on a clean stop.

### Live SSE load-shed (slow subscriber)

The LiveRunner publishes to a `tokio::broadcast` channel. A subscriber that falls
more than the buffer (256) behind receives a `Lagged` error for **its own
receiver only** — the producer never blocks and other subscribers are unaffected.
The monotonic per-event sequence number makes the skipped range visible, which is
exactly the gap the `Last-Event-ID` resume contract reports.

## Soak test

`crates/nexus-api/tests/soak/backpressure_soak.rs`, `#[ignore]`-by-default (the
sanctioned opt-in run). Invoke:

```
make -C nexus soak
# or
cd nexus/backend && cargo test -p nexus-api --features testing \
    --test routes_soak_backpressure -- --ignored --nocapture
```

Needs a running Docker daemon (it stands up its own Postgres via testcontainers).

| Tunable | Default | Purpose |
|---|---|---|
| `NEXUS_SOAK_SECS` | 20 | Steady-case run length. Set `600` for the ≥10-minute soak. |
| `NEXUS_SOAK_BATCH` | 5000 | Rows per source emit. |

Cases:

- **`steady_high_rate_is_bounded_and_lossless`** — high-rate finite flow into a
  Postgres datasource; samples process RSS across the run (must stay within
  512 MB of baseline — proves no unbounded growth), asserts count-in ==
  count-landed (zero loss), and that a flush time was recorded (flush latency is
  observable).
- **`fat_batch_is_sliced_and_bounded`** — a single 2,000,000-row emit must be
  sliced by `max_batch_rows` and stay within the RSS bound; all rows land.
- **`db_fault_surfaces_error_without_silent_loss_under_halt`** — drops the
  destination table mid-run so writes fail; under `halt` the flow surfaces the
  error and stops (no panic, no infinite retry), `write_errors` is counted, and
  `rows_written` reconciles against what landed before the fault.

Runtime: the default 20s steady case plus the fat-batch and chaos cases complete
in a few minutes on a laptop with Docker; the ≥10-minute variant
(`NEXUS_SOAK_SECS=600`) is the documented long soak.
