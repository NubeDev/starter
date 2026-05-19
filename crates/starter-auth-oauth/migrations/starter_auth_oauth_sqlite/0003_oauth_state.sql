-- starter-auth-oauth migration 0003: shared state store for the
-- short-lived OAuth flow record. The in-memory default
-- (`MemoryStateStore`) is fine for a single-node deploy, but the
-- moment a second instance comes online the user can start the flow
-- on instance A and have the provider 302 the browser to instance B
-- with the `state` parameter — and B has no entry to consume. This
-- table is the cross-instance handoff: any node can `put` and any
-- node can `take`.
--
-- Schema mirrors the in-memory `OAuthFlowState` struct one-to-one so
-- the SQL impl is the same shape and there is no field that exists
-- only in the durable backend. `state` is the natural PK (it is the
-- single-use random token the provider echoes back; the probability
-- of a 32-byte collision is negligible).
--
-- No FK to `starter_auth_users_users` on `link_mode_user_id` — the
-- callback handler validates the user id when it consumes the flow
-- and a stale user row should not be able to take the state-store
-- write path offline.
--
-- TTL eviction is the consumer's job (see SqliteStateStore::take);
-- there is no background sweeper. A bounded grow path is fine
-- because `take` is also the sweep.
CREATE TABLE IF NOT EXISTS starter_auth_oauth_state (
    state             TEXT NOT NULL PRIMARY KEY,
    provider          TEXT NOT NULL,
    pkce_verifier     TEXT NOT NULL,
    return_to         TEXT,
    link_mode_user_id TEXT,
    created_at        TEXT NOT NULL
);

-- The sweep predicate inside `take` walks expired rows; without
-- this index that walk is a full scan once the table grows past a
-- few thousand in-flight entries.
CREATE INDEX IF NOT EXISTS idx_oauth_state_created_at
    ON starter_auth_oauth_state(created_at);
