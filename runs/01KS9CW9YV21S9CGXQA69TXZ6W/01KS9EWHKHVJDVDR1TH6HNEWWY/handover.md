## Done

- Created crates/starter-warehouse/ behind default-off `warehouse` cargo feature (with sub-features `testing` and `mcp`); added to workspace members and workspace dep table.
- Implemented every W9 node kind as NodeBehavior impls plus NodeDescriptor consts: tap_write, curate_write, bulk_import (target required, no default), sandbox_define/redefine/drop, cleaner_define/promote/drop, mart_define/read/promote/drop. `nodes::descriptors()` is the aggregate consumers register.
- `nodes::runtime::WarehouseRuntime` is the shared body — REST handlers and node bodies share one implementation. Mechanically enforces W5 (definition_hash idempotency, generated AggregatingMergeTree DDL with D7 ORDER BY), W11 (FreshnessProbe envelope + 503 on failed_refresh), W12 (Author classification + ext re-quarantine in the same txn + live-mart quota), W13 (dictGetOrNull in generated read query), W14 (filter validation vs promoted_columns with typed RuntimeError variant), W15 (two-window catalog GC daemon), W16 (async_insert_oldest_age_ms surfaced via /api/warehouse/status).
- RF-4 sandbox.redefine refused when frozen_at_revision is set; RF-6 cleaner sync→async auto-promotion past 1M rows or 5 min wall clock.
- REST surface under /api per SCOPE table: /api/marts (POST/DELETE/promote), /api/marts/:name/data with structured 400 body naming promoted_keys, /api/sandboxes, /api/warehouse/{gc,status,audit}. SSE routes /api/marts/events + /api/entities/events use starter-server::sse::keep_alive(15s).
- MCP tool surface (mcp.rs) behind `mcp` sub-feature exposes the seven SCOPE tools (query_entities, tag_entity, define_mart, drop_mart, read_mart, define_sandbox, peek_sandbox).
- Tests: 20 pure-logic tests green (W5/W12/W14 invariants, DDL identifier validation, deterministic-key rule, sandbox TTL); 6 testcontainer-backed integration tests marked #[ignore] covering W5 idempotency, W14 400, W12 re-quarantine, RF-4 freeze, RF-6 auto-promote, W11 envelope. `cargo test -p starter-warehouse --features 'warehouse mcp testing'` is green (containers skipped).
- Committed as `723e812` "stage 4 (slice D) — starter-warehouse capability crate".

## Next

- Stage 5 (slice E): flow-agent smoke + iot-anomaly-detector port that demonstrates the worked example end-to-end (MQTT flow + cleaner.define + mart.define + mart.read), then final sweep.
- Stage 5 should also run the `--ignored` testcontainer suite against docker to produce the REVIEW gate 2 transcript (W14 400 body, W12 re-quarantine SELECT, RF-4 refusal, RF-6 auto-promote, W11 503 status, W16 async_insert_oldest_age_ms bound).

## What you need to know

- Crate compiles under `--features warehouse`, `--features 'warehouse testing'`, and `--features 'warehouse mcp testing'`. Workspace-wide `cargo check --workspace` failed on unrelated AWS-SDK MSRV (rustc 1.91 required for aws-smithy-* crates) — not introduced by this stage; the starter-warehouse subtree and its full dep closure resolve fine.
- W9 says "descriptor registration via starter-flow-nodes". To avoid a circular dep, `starter-warehouse::nodes::descriptors()` is the canonical list — a consumer crate folds it into starter-flow-nodes' `StaticNodeKindRegistry` builder. No changes to starter-flow-nodes itself.
- The runtime's `mart_read` returns an empty `rows` vec; the W14 validation gate is exercised, but the actual ClickHouse SELECT path is a TODO surface for stage 5 once the iot example needs real rows. `ddl::mart::read_query` builds the SELECT shape (with `*Merge()` + `dictGetOrNull` per W13); wire it in stage 5.
- The audit module hashes only (filter, group_by, aggregations) — time_bucket_secs is part of the `MartSpec::definition_hash` but a PgInterval round-trip from the audit row is awkward. Drift on (filter, group_by, aggregations) is what catches the realistic mutation; the time_bucket CHECK constraint protects the column itself.
- SSE routes are scaffolded with empty streams; stage 5 should wire them to the flow bus's `mart.defined / mart.promoted / mart.dropped` / `entity.*` event streams.
- bulk_import opens an insert via `self.ch.inner().insert(table)` for non-`samples` targets — `async_insert=0` override per W8a is documented but not yet flipped on the client; the inserts go through the same `ChClient` whose default is `async_insert=1`. Stage 5 should add a `with_option("async_insert", "0")` per-call override for the bulk path to honour W8a in full.
- Live-mart quota uses application-layer probe + the existing partial-index trigger from migration 0005; the catalog crate's `promoted_columns TEXT[]` column is expected on `marts` — UPDATE is wrapped in `.ok()` so a dev schema without it still works.

## Open questions

- Spec gap: SCOPE talks about "starter-server or axum" — chose axum directly (axum + starter-server::sse::keep_alive). Confirm at REVIEW gate 2 that this is acceptable, or wrap the router behind a starter-server::Router builder if the convention is stricter.
- Spec gap: the cleaner crate's `validate_entity` enum is parsed in the cleaner_define node but not yet enforced at write time — `strict_via_postgres` would require a per-row PG roundtrip inside the CH MV which the spec itself notes is impossible. Open question: should `strict_via_postgres` reject at define-time, or should the warehouse expose a `curate.write`-style alternative for the strict path? Currently the runtime accepts the enum verbatim; the storage crate's `ValidateEntity::Strict` variant exists but is unused.
- The W12 re-quarantine txn currently happens inside `mart_define`'s transaction — confirm at REVIEW gate that the operator UI/CLI shows the re-quarantine as a separate audit event, not just a side effect of the new mart insert.
