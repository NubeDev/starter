# RW-04 — Any-DB store: sinks target a datasource id

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-03 (native engine at HEAD). Builds on WS-08b datasource-kinds
> (`nexus-api/src/datasource_kinds/**`, `nexus-api/datasource-kinds/` pack).

## Current state

- `sink/postgres.rs` is hardwired to Postgres: config carries connection info, rows go
  Arrow→JSON→parameterized INSERT one batch at a time.
- Datasources already exist as tenant records with envelope-encrypted creds
  (`nexus-store/src/datasource/secret.rs` ~63-89 — `Envelope::seal/open`) and a
  declarative kind format from WS-08b (postgres query / mqtt stream kinds).
- Flows reference raw sink configs, not datasource records.

## Scope

1. `sink/datasource.rs` — a sink whose config is `{ "datasource": "<id>", "table": "…",
   "batch_rows": N?, "batch_ms": T? }`. It resolves the datasource record (tenant-scoped),
   decrypts creds via the existing envelope path (audit-logged like the query path), and
   dispatches to a writer by datasource kind.
2. Writer trait `DatasourceWriter { async fn write_batch(&mut self, &RecordBatch);
   async fn flush(&mut self) }` + the first two impls:
   - `postgres` (covers Timescale): use tokio-postgres `BinaryCopyInWriter`
     (`COPY … FROM STDIN BINARY`) — that is the right primitive; do not reinvent
     multi-row INSERT batching and call it done. Keep the existing Arrow→param mapping
     rules; fall back to multi-row INSERT only for types COPY BINARY can't carry.
   - `file`: Parquet part-files via `parquet::arrow::AsyncArrowWriter` + `object_store`
     (local-fs first, S3 config accepted but may be feature-gated). Small-files guard:
     the Parquet writer rotates on SIZE/ROWS (≥64MB or N-minutes, whichever first), NOT
     the generic batch timer — timer-flushed tiny row groups are what makes Parquet slow.
   Registered per kind so RW-07 extensions can add more.
3. Batching: accumulate until `batch_rows` OR `batch_ms` (tokio interval), flush on either,
   always flush on `close()`/cancel. This is the write-side backpressure contract RW-08
   soak-tests.
4. Keep `sink/postgres.rs` as a thin alias over the new path (stored configs name it) —
   same JSON config keeps working; internally route to the datasource writer machinery
   or the legacy direct-connection mode when raw conn config is supplied.
5. Migration `17xx` only if a new column/table is genuinely needed (likely none —
   datasource records already exist).

## Acceptance

- A flow `{input: simulator} → {output: datasource{id, table}}` lands rows in a docker
  Postgres via the tenant datasource record — e2e test (docker-gated like existing ones).
- File writer produces readable Parquet (read back via DataFusion in the test).
- Batch flush triggers on both row-count and timer paths (unit tests with paused time).
- Old-style postgres sink configs still run unchanged (fixture test).
- Decrypt audit fires for sink-side cred access, same as query-side.
