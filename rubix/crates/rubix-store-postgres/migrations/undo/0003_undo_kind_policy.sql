-- Per-kind retention policy for `undo_snapshots`.
--
-- Today the sweep in `rubix-agent::boot::undo_sweep` applies one
-- global pair of limits to every `(tenant_id, resource_kind,
-- resource_id)`. Different kinds want different curves:
--
--   - high-value, low-frequency kinds (e.g. `user`, `team`) deserve
--     a longer retention so an operator demote/disable from last
--     week is still undoable;
--   - chatty kinds (e.g. `flow_def` after a noisy edit session)
--     should trim faster so the table does not accumulate
--     hundreds of near-identical YAML snapshots per page.
--
-- Encoding the curves as branches in Rust is the wrong shape — it
-- couples retention policy to a deploy. Storing the policy lets an
-- operator tune it via SQL (or a future admin verb) without a
-- rebuild, and lets new kinds inherit defaults silently.
--
-- The sweep query JOINs this table on `resource_kind`. Kinds with
-- no row fall back to the boot-config defaults (`[undo]
-- max_rows_per_resource`, `[undo] max_age_days`); this is the
-- "additive migration" property — adding a new kind never requires
-- seeding a policy row.

CREATE TABLE IF NOT EXISTS undo_kind_policy (
    resource_kind         TEXT    PRIMARY KEY,
    max_rows_per_resource INTEGER NOT NULL,
    max_age_days          INTEGER NOT NULL,
    CONSTRAINT undo_kind_policy_rows_positive  CHECK (max_rows_per_resource > 0),
    CONSTRAINT undo_kind_policy_age_positive   CHECK (max_age_days          > 0)
);

-- Seed the curves the proposal calls out as worth deviating from
-- the defaults. These are the only kinds with bespoke policy
-- today; everything else inherits `[undo]` defaults.
--
-- `user` and `team` get the longer 180d curve because an "I
-- demoted the wrong person last week" recovery is the canonical
-- UX win — security-grade auditability still belongs to the audit
-- log (see proposal §3.3), not this retention window.
--
-- `flow_def` gets the shorter 30d / 200-row curve because deploys
-- accumulate quickly during goal-3 authoring sessions and old
-- revisions are recoverable from the changelog row anyway.
INSERT INTO undo_kind_policy (resource_kind, max_rows_per_resource, max_age_days)
VALUES
    ('user', 50, 180),
    ('team', 50, 180),
    ('flow_def', 200, 30)
ON CONFLICT (resource_kind) DO NOTHING;
