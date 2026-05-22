## Done

- Added `flow` feature on `crates/starter-store-postgres/Cargo.toml` enabling `dep:starter-flow-spi`, `dep:uuid`, and `sqlx/chrono`+`sqlx/json`+`sqlx/uuid` (chrono was already unconditional for agent-session; new bits are uuid, sqlx/uuid, and the json/chrono flags reaffirmed at the flow gate).
- Landed `crates/starter-store-postgres/src/flow/` behind `#[cfg(feature = "flow")]`: `mod.rs` (re-exports + `FLOW_MIGRATOR` / `FLOW_MIGRATION_SOURCE`), `schema.rs` (JSON `to_value`/`from_value` + `sqlx_to_flow`/`sqlx_backend` helpers), `flow_store.rs` (PgFlowStore : FlowStore), `run_store.rs` (PgRunStore : RunStore), `session_store.rs` (PgSessionStore : SessionStore).
- Wrote `crates/starter-store-postgres/migrations/flow/0001_init.sql` — a single consolidated Postgres rewrite of the SQLite twin's `0001` + `0002`. JSONB for every json column, `TIMESTAMPTZ NOT NULL DEFAULT NOW()` for timestamps, BIGINT seq, partial UNIQUE on (service_name, dedup_key), explicit `ON CONFLICT (...)` targets on head/session upserts.
- Ported the SQLite suite into three test files: `tests/flow_store.rs`, `tests/run_store.rs`, `tests/session_store.rs`, all `#[cfg(all(feature = "flow", feature = "testing"))]` and `#[ignore = "requires docker"]`, booting via `testing::with_database()`.
- Wired `pub mod flow` in `src/lib.rs` under `#[cfg(feature = "flow")]`.
- `cargo check`/`cargo test --no-run`/`cargo clippy --tests -D warnings`/`cargo fmt --check` all green on `-p starter-store-postgres --features "flow testing"`.
- Committed as `stage 1 (slice A) — starter-store-postgres::flow` on `codeless/flow-agent-postgres-only`.

## Next

- Stage 2 (slice B): port `agent_session_store.rs` (already present on the Postgres side under `agent_session/` — confirm parity vs. the 716-LOC SQLite source) and ensure the flow + agent-session features compose cleanly for flow-agent. Then REVIEW gate before slice C.

## What you need to know

- `crates/starter-store-postgres/src/agent_session/` and `migrations/agent_sessions/` already exist (pre-job work). Slice B should audit them against `starter-store-sqlite/src/flow/agent_session_store.rs` rather than write a new one from scratch — the job brief assumes a port not yet started, but the file is already on disk.
- Workspace-wide `cargo test --workspace` currently fails with a rustc 1.91 vs 1.90 toolchain mismatch on aws-smithy-* crates. This is **pre-existing** (reproduced on a clean stash before any of my changes) and unrelated to slice A. The slice A acceptance command (`cargo test -p starter-store-postgres --features "flow testing"`) builds clean.
- Schema deviation from the SQLite twin: ids stay `TEXT` (mirrors SQLite binding shape via `Uuid::to_string()`), seq is `BIGINT` not `INTEGER`, every JSON column is `JSONB` and bound as `serde_json::Value` (not `String`). No booleans in this schema, so the BOOLEAN translation rule from ADR-3 didn't apply to slice A.
- Re-puts of an existing `(flow_id, revision_id)` use `ON CONFLICT … DO NOTHING` instead of SQLite's `INSERT OR IGNORE`. Behaviourally identical; added a test (`flow_put_is_idempotent_on_existing_revision`) to lock the body-immutability invariant.
- The `checkpoint_atomicity_failed_tx_preserves_prior_state` test inserts via `'…'::jsonb` casts; literal text would be rejected by Postgres' typed JSONB columns.
- `DOCS/storage/ADR-001-flow-agent-postgres-only.md` referenced in the job brief does **not** actually exist in the repo. The authoritative source used was `.codeless/jobs/flow-agent-postgres-only/SCOPE.md`.

## Open questions

- Should `FLOW_MIGRATION_SOURCE.name = "flow"` (matches SQLite). Currently the agent-session source uses `name = "agent_sessions"` — slice B will need to confirm both can coexist on the same pool's migrate chain (they target different migration tables already, but flow-agent's wiring should be sanity-checked when it lands).
