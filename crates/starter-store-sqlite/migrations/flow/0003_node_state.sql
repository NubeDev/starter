-- Stage A+B.1: per-node persistent state (NodeStateStore SPI seam).
-- See DOCS/flow/scope/node-state.md.
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
    value      BLOB NOT NULL,
    version    INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (flow_id, node_id, key)
);
