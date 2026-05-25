-- Stage A+B.1: per-node persistent state (NodeStateStore SPI seam).
-- Postgres twin of `crates/starter-store-sqlite/migrations/flow/0003_node_state.sql`.
-- See DOCS/flow/scope/node-state.md.
--
-- Dialect translation rules applied uniformly (mirrors 0001_init.sql):
--
--   * `BLOB`                -> `BYTEA`
--   * `INTEGER` (i64 use)   -> `BIGINT`
--   * `TEXT` timestamps     -> `TIMESTAMPTZ NOT NULL DEFAULT NOW()`
--   * `CURRENT_TIMESTAMP`   -> `NOW()`
--
-- (flow_id, node_id, key) is the primary key. `value` is opaque bytes
-- supplied by the node body — the store is content-agnostic. `version`
-- is a monotonically-bumped u64 the store assigns on every successful
-- put / cas; the column is the compare-and-swap target.
--
-- Forward-only convention: never destructively rewrite. The starter-flow
-- engine deletes rows on revision change when the kind opts into
-- `reset_on_redeploy` (the engine's job, not the store's).

CREATE TABLE IF NOT EXISTS node_state (
    flow_id    TEXT NOT NULL,
    node_id    TEXT NOT NULL,
    key        TEXT NOT NULL,
    value      BYTEA NOT NULL,
    version    BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (flow_id, node_id, key)
);
