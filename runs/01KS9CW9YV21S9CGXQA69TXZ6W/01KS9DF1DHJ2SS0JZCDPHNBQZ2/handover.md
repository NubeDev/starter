## Done

- Added default-off `dimensions` feature to `starter-store-postgres` (Cargo.toml + lib.rs export of `DIMENSIONS_MIGRATION_SOURCE`).
- Landed 8 migrations under `migrations/dimensions/` (entities, entity_refs, tag_definitions, tag_prefix_registry, marts catalog w/ partial-index live-quota trigger, cleaners catalog, sandboxes catalog, ext_manifest_approvals) — registered on the dedicated `_sqlx_migrations_dimensions` version table via the existing namespaced migrate runner.
- Wrote typed CRUD modules under `src/dimensions/`: `entities`, `entity_refs`, `tag_definitions` (bridged to `starter_tags::TagDefinition` / `TagKind`), `tag_prefix_registry` (with `RegisterError::Conflict` and a `register_in_tx` helper for the BI-4 install-txn flow), `marts`, `cleaners`, `sandboxes` (with `redefine_columns` revision bump + `freeze` for promote), `ext_manifest_approvals`, plus `catalog_gc` (W15) and `catalog_audit` (W5 drift probe).
- Integration tests under `tests/dimensions_basic.rs`, `tests/dimensions_prefix.rs`, `tests/dimensions_marts.rs` cover: 8 migrations apply, FK CASCADE on entity_refs, TagDefinition round-trip, idempotent approvals, T6 BI-4 prefix-conflict txn failure (high-level + in-tx), prefix shape CHECK rejections, live-mart quota only counts live rows (200 quarantined rows do not inflate count, 4th live insert trips, freeing a slot via status transition unlocks).
- `cargo build` and `cargo clippy --tests` for `-p starter-store-postgres --features "dimensions testing"` are clean.
- Commit `stage 2 (slice B) — starter-store-postgres dimensions feature`.

## Next

- Stage 3 (slice C): create `starter-store-clickhouse` crate with raw_events / samples / events / documents tables, entities_dict dictionary, async insert discipline (W8).

## What you need to know

- `starter-store-sqlite` was deliberately not touched (per the job spec).
- `MartRow.time_bucket` is `sqlx::postgres::types::PgInterval`, which does not implement serde — `MartRow` is therefore `Clone + Debug + FromRow` only. `starter-warehouse` will need to convert to its own serialisable shape if it wants to ship the row in JSON envelopes.
- The `dimensions/` module silences `missing_docs` at the module level (`#![allow(missing_docs)]`) — column-level docs are kept in the migration SQL where they belong; re-stating them on every Rust field was noise.
- The mart live-quota is read from the Postgres GUC `warehouse.live_mart_quota` (default 50). Tests lower it to 3 via `set_config(..., false)`. Per-deployment override is `ALTER DATABASE ... SET warehouse.live_mart_quota = N` or a startup `SET` from the orchestrator.
- The `cleaners.validate_entity` column accepts `'strict'` at the CHECK level, but the SCOPE narrative requires `cleaner.define` to reject `Strict` at node-call time (dictionary lag makes it implementable only in `curate.write`). The DB-level CHECK lets the column hold the policy decision; the orchestrator owns the rejection.
- Tests are all `#[ignore = "requires docker"]` consistent with the existing crate convention; run with `cargo test -p starter-store-postgres --features "dimensions testing" -- --ignored` against a host with Docker.
- `cargo check --workspace` fails on a pre-existing rustc 1.91 / aws-sdk MSRV mismatch unrelated to this stage; the crate-local build/clippy/test compile cleanly.

## Open questions

- The job spec references "Tags T6 BI-4" and "Warehouse RF-4" section markers that do not appear verbatim in the SCOPE docs as currently checked in. The prefix-registry table (BI-4) and the sandbox columns_revision/frozen_at_revision pair (RF-4) were implemented from the surrounding narrative; a future doc pass may want to add the explicit anchors.
- `catalog_audit::marts_for_audit` returns the raw fields for hash recomputation but does not itself recompute the hash — the canonical hash function lives in `starter-warehouse` (stage 4), so the audit loop is owned there.
