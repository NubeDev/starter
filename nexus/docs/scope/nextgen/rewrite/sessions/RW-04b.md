# RW-04b — File-datasource persistence + stream-connector probes (RW-04 fix pass)

Status: Done
Started: 2026-06-10 06:20 UTC
Finished: 2026-06-10 07:30 UTC

## Scope

A fix pass with no standalone spec; scope is the **Proposed** sections of two
TODOs.md entries, both assigned to RW-04's lane:

1. "RW-05 — File datasource (parquet/csv) cannot be *persisted*": the store schema
   was rigidly Postgres-shaped, so a file kind could not be stored and
   `federation::resolve::resolve_one` returned `Invalid` for it.
2. "RW-09 — Zenoh store-side connect probe not implemented": `test_connection` had
   no zenoh arm (and mqtt was also catalogue-only).

## First action — drift re-grep

Re-greped every file:line the two TODOs cited; all claims held against HEAD:
- migration `0001_tenancy.sql:21-38` — `nexus_datasources` host/port/database/
  db_user + the four secret columns all `NOT NULL`, no `config`/`path` column.
- `nexus-api/src/federation/resolve.rs::resolve_one` — file arms folded into the
  catch-all `other => Invalid` with the deferral comment.
- `routes/datasources/test_connection.rs` — `DatasourceKind::Postgres` the only
  probe arm; `DatasourceKind` enum (spi `dto/datasource/shared.rs`) had only
  `Postgres`.
No spec doc needed a `Verified:` bump (the TODOs *are* the spec; their pointers
were accurate).

## What landed

### File-datasource persistence (store lane)
- Migration `2001_datasource_file_config.sql` (20xx block, RW-04's reserved lane;
  `2101`/`2201` already taken so `20xx` was free): relax host/port/database/
  db_user + the four secret columns to nullable, add `config jsonb`. Existing
  Postgres rows unaffected (populated columns, NULL config).
- `record.rs`: `DatasourceRecord.config: Option<Value>`; `NewDatasource.secret:
  Option<String>` + `NewDatasource.config: Option<Value>`.
- `insert.rs`: seal only when a secret is present; secret-less rows bind NULL
  secret columns and `key_version = 0` (COALESCE).
- `fetch.rs`: select + decode `config`; read the now-nullable connection columns
  as `Option<_>::unwrap_or_default()`.
- `federation::resolve::resolve_one`: `parquet`/`csv` arms build
  `FederatedSource::{Parquet,Csv}` from `record.config.path` (+`has_header` for
  csv). A file row with no `config.path` is a loud `Invalid`, never a silent drop.
- Non-test caller `routes/datasources/create.rs` + reverse-snapshot decode
  `reversible/datasource.rs` updated for the new fields (create stays Postgres,
  `secret: Some(..)`, `config: None`).

### Stream-connector probes (RW-04 test_connection lane)
- `DatasourceKind` gains `Mqtt` + `Zenoh` (snake_case); `convert.rs::kind_to_stored`
  maps them.
- `TestConnectionRequest`: SQL fields made `Option` + add `config: Option<Value>`
  (additive/relaxation — old postgres payloads still deserialize). Drops `Eq`
  (Value is not Eq).
- New store `datasource/zenoh/{mod,probe}.rs`: feature-gated (`zenoh`, OFF by
  default, mirrors `mqtt`) short-lived `zenoh::open` reachability probe; feature-off
  stand-in returns a clear "not enabled" error.
- `test_connection.rs`: dispatch `Postgres|Mqtt|Zenoh`; mqtt/zenoh read their
  params from `config`; shared `shape()` maps the store result to the wire outcome.

### Contract regen
- openapi.json regenerated (+30/-15, add-only/relaxation) + `pnpm codegen`.
- UI `DatasourceFormDialog.tsx`: `connectionBody()` returns a concrete (non-null)
  Postgres object so the create body stays satisfied after the probe DTO loosened.
  Forced codegen-drift fix, mechanical.

### Tests
- `federation_e2e_test.rs`: new `stored_parquet_joins_live_postgres_end_to_end`
  (#[ignore]=docker) — writes a Parquet fixture via arrow/parquet (new test-only
  dev-dep `datafusion`, already in tree), persists it as a secret-less `parquet`
  datasource through the store, and joins it against a live PG datasource in one
  federated SQL — the previously-missing stored-Parquet ⋈ PG leg.
- `crud_test.rs` + the four other `NewDatasource` test call sites updated for the
  new field shape.
- zenoh probe inline tests: feature-off "not enabled"; feature-on peer-mode open
  (needs `multi_thread` rt — zenoh refuses the current-thread scheduler).

## Verification
- `cargo test --workspace` green (no failures across all binaries).
- `cargo test -p nexus-store --lib --features zenoh` green (6/6).
- `cargo tree -p nexus-store -i zenoh` = 0 deps by default; 1 under `--features zenoh`.
- openapi add-only; `pnpm codegen` + `pnpm typecheck` green.

## Assumptions / deviations
- Kept the REST **create** DTO Postgres-shaped: the authoritative Proposed scope is
  store-layer persistence + resolve + e2e, not a new create-by-file REST shape. A
  file datasource is persisted via the store (as the e2e does); a file-aware create
  DTO is a separate follow-up if/when a UI needs it.
- The store `resolve_sink_config` (write path) was left as-is: file kinds are
  read-only federation inputs, not datasource-id sink targets, so its catch-all
  stays correct.
- `mqtt` probe was wired into `test_connection` for parity alongside zenoh (the
  RW-09 TODO names "mqtt-parity"); the store mqtt probe already existed.

## Follow-ups
- None new logged. The pre-existing out-of-lane test drift entries (RW-06/RW-08)
  remain owned by their lanes.
