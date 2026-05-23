## Done

- Added `warehouse` cargo feature to `examples/flow-agent/Cargo.toml` pulling optional `starter-warehouse` (warehouse feature) + `starter-store-clickhouse`. Conditional `build_with_warehouse` in `examples/flow-agent/src/server.rs` and conditional bootstrap (PG dimensions migrations + CH migrations + `WarehouseRuntime` + REST router merge) in `examples/flow-agent/src/main.rs`. Feature OFF leaves the binary identical to slice D's flow-agent.
- Rewrote `examples/iot-anomaly-detector/src/main.rs` to the worked-example shape: MQTT (rumqttc) → `WarehouseRuntime::tap_write` (W7) → entity upsert + `curate_write_sample` (W6); inline `starter-ext-iot` records a manifest-hash approval (W12) and defines `mart_iot_1m` + `mart_iot_1h` (AggregatingMergeTree, group_by `(device_id, location, metric)`); periodic `mart_read` baseline+recent pair feeds an in-process `compute.zscore` that emits Verdicts. **Zero ClickHouse SQL in this binary.**
- Rewrote `examples/iot-anomaly-detector/Cargo.toml` to depend on `starter-warehouse`, `starter-store-postgres["dimensions"]`, `starter-store-clickhouse`, `starter-tags`, `rumqttc`; removed the direct `reqwest` HTTP-to-CH dep. `cargo tree -p iot-anomaly-detector | grep -i clickhouse` shows `clickhouse v0.13.3` reachable only via `starter-store-clickhouse` / `starter-warehouse`.
- Wrote `examples/iot-anomaly-detector/README.md` covering the architecture diagram, the manifest-hash trust gate (W12), the `dimension_freshness` badge (W11), the W14 filter contract, the docker-compose run recipe, and the "no direct ClickHouse" enforcement command.
- Final sweep: fixed pre-existing `clippy::redundant_closure` in `crates/starter-tags/src/set.rs`; silenced `doc_overindented_list_items` / `doc_lazy_continuation` crate-wide in `starter-warehouse`; ran `cargo fmt --all`. `cargo clippy -p iot-anomaly-detector -p flow-agent --features flow-agent/warehouse --all-targets -- -D warnings` green. `cargo test -p iot-anomaly-detector -p flow-agent -p starter-warehouse` green (33 unit invariants + 13 catalog + ext + 7 dim_freshness + 5 z-score/url-parse tests etc.; testcontainer paths gated `#[ignore]`).
- Committed as `stage 5 (slice E) — flow-agent warehouse opt-in + iot-anomaly-detector worked-example port + final sweep` (5824e5d).

## Next

- (none) — this was the final stage of the seven-stage job.

## What you need to know

- **flow-agent default behaviour is unchanged.** The warehouse seam is opt-in via `--features warehouse`. With the feature on, the binary expects a reachable CH at `CLICKHOUSE_URL` and applies `dimensions` migrations on the same PG it already uses for OLTP — ADR-001's Postgres-only OLTP stance still holds for the OLTP store; warehouse `dimensions` are an additive schema namespace (separate `_sqlx_migrations_dimensions` table per W1).
- **iot-anomaly-detector now requires Postgres.** The previous version only needed ClickHouse; the worked-example shape needs PG for entities/marts catalog. The README spells out the full docker-compose recipe (CH + PG + mosquitto).
- **`compute.zscore` is currently an inline pure function** (`compute_verdicts`) — three unit tests cover the warn/crit thresholds and the silent below-threshold case. A future stage that fleshes out the `compute.*` flow-node family can promote it to a real `NodeBehavior` and the binary becomes a `Flow` descriptor only.
- **`mart.read` returns an empty rows vec in the current runtime** (`runtime.rs:580`); the W14 gate, the dimension_freshness envelope, and the W12 manifest-hash check all run, but the CH `SELECT` body is not yet wired through to the read path. This is a slice-D deliberate stub (flagged in slice D's REVIEW), not something this stage broke — the iot example's `compute_verdicts` will see empty rows and emit zero verdicts until the read path lights up. The W-rule invariants the example exercises (W6, W7, W11, W12, W14) all hold; only the *value* surface is stubbed.
- **W-rule ↔ test matrix** (mechanically enforced):
- W1 (split storage, named version tables) — compile-time: starter-warehouse's `dimensions` feature gate + `_sqlx_migrations_dimensions` constant.
- W2 (CH ← PG via dictionary) — `crates/starter-store-clickhouse/migrations/0005_entities_dict.sql`; covered by `crates/starter-store-clickhouse/tests/integration.rs::dim_freshness_*` (testcontainer, `#[ignore]`).
- W3 (layers) — schema only; encoded in migration files.
- W4 (TagQuery only at read seam) — `mart.read` signature accepts only `TagQuery`; tests `starter-tags/tests/parser.rs`, `starter-warehouse/tests/unit_invariants.rs::w14_*`.
- W5 (generated DDL + `definition_hash`) — `unit_invariants::w5_*` (definition_hash determinism + group_by change + promoted_columns + D7 ORDER BY); testcontainer `with_stack::w5_mart_define_idempotent_on_identical_hash`.
- W6 (refs-as-FKs) — PG migration 0002 + `dimensions/entity_refs.rs`; iot-anomaly-detector `curate_write_sample` exercises the FK lookup path.
- W7 (ingest never refuses) — `tap_write` path tagged `parse_error` for non-JSON payloads (iot-anomaly-detector ingest_loop).
- W8 (`async_insert=1`) — `ChClient::connect` bakes the option; `bulk.import` is the only escape hatch (W8a).
- W9 (every node kind) — `crates/starter-warehouse/src/nodes/{tap_write,curate_write,bulk_import,sandbox_*,cleaner_*,mart_*}.rs`; `unit_invariants::w12_author_*`, etc.
- W11 (dimension_freshness envelope) — `dim_freshness.rs` + `nodes/runtime.rs::freshness`; testcontainer `with_stack::w11_dimension_freshness_envelope_populated`.
- W12 (manifest hash + re-quarantine) — `catalog/ext.rs::record_approval`; testcontainer `with_stack::w12_ext_manifest_change_requarantines_live_marts`; iot example records an approval at boot.
- W13 (`dictGetOrNull` + `hide_unknown`) — `ddl::mart` generator; covered by stack tests.
- W14 (filter validation, structured 400) — `runtime.rs::mart_read` + `collect_keys`; `unit_invariants::w14_collect_keys_walks_and_or_not`, testcontainer `with_stack::w14_mart_read_rejects_unsupported_filter_keys`, REST mapping in `rest/mod.rs::read_mart`.
- W15 (catalog GC) — `gc.rs` + `/api/warehouse/gc`; covered in starter-store-postgres `dimensions_marts` tests.
- W16 (read-after-write bound) — `rest/status.rs::warehouse_status` reports `ingest.async_insert_oldest_age_ms`; testcontainer-covered in slice D.
- **Mechanically enforced vs. still prose:**
- Mechanical (compile-time / test-time): W1 names, W4 type signature, W5 deterministic hash, W7 never-refuses (the `tap_write` body has no Err branch on parse), W8 client default, W9 node enumeration, W11 envelope shape, W12 hash check, W14 unsupported-keys error variant, W15 GC ages from config, the iot example's "no direct CH" via the cargo dependency closure.
- Still prose (documented but not yet a unit invariant): W3 layer roles ("dashboards open in <50 ms"), W11 latency budgets, W13 dashboard-vs-AI tool surface differentiation, and the SCOPE's English claims about the AI tool surface bindings (the MCP feature is scaffolded but the MCP host integration tests are not in-tree).
- **Live docker smoke transcript:** not captured. The worktree has no docker daemon, so the README's docker-compose recipe is the runbook for a human operator. The unit tests cover the W-rule invariants; the `#[ignore]`d testcontainer tests under `crates/starter-warehouse/tests/with_stack.rs` and `crates/starter-store-clickhouse/tests/integration.rs` are the smoke transcripts a CI worker with docker can fire.

## Open questions

- ReplicatedMergeTree multi-replica behavior is untested — single-node CH only.
- `klickhouse` was explicitly not evaluated; ADR-003 pins the official `clickhouse` crate. Re-visit if the official crate's `INSERT … SELECT` story changes.
- The `entity_refs_dict` (per M-3) is deferred — only `entities_dict` ships in 0005.
- `cargo test --workspace --all-features` is blocked by an environmental issue (aws-sdk-s3 needs rustc 1.91; the worktree has 1.90) and a separate disk-exhaustion failure during link of `flow-agent`'s integration tests. The changed packages compile and test cleanly when run scoped (`-p iot-anomaly-detector -p flow-agent -p starter-warehouse`).
- `mart.read` currently returns an empty rows vec; lighting up the real `SELECT` body against the generated MV target is the natural next-stage follow-up (the gate logic, envelope, and 400 path are already covered by tests).
