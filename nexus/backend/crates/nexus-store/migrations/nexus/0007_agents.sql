-- AI agents and their sessions. An *agent* is a saved configuration the control
-- plane drives through the nexus-ai facade: which backend (inference provider or
-- coding agent), which model, a system prompt, and an opaque config blob. A
-- *session* is one conversation/run against an agent: a status, the prompt that
-- started it, and the accumulated transcript. Both are tenant-scoped and
-- RLS-isolated exactly like flows/dashboards; running state lives in memory keyed
-- on the immutable id, the rows are the durable record.

CREATE TABLE nexus_agents (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    name          text NOT NULL,
    -- Which facade tier/backend drives this agent. For inference: a provider hint
    -- (e.g. "anthropic"); for the agent tier: a coding-agent backend ("claude",
    -- "codex"). Opaque to the store; the nexus-ai Client interprets it.
    backend       text NOT NULL,
    -- The model reference: a concrete id ("claude-opus-4-8") or a size alias
    -- ("small"/"medium"/"large"). Resolved by the facade's AliasMap at run time.
    model         text NOT NULL DEFAULT 'large',
    -- Optional system prompt prepended to every session's messages.
    system_prompt text,
    -- Provider/agent-specific knobs (temperature, max_tokens, worktree isolation,
    -- …). Opaque jsonb; validated when a session runs, not here.
    config        jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at    timestamptz NOT NULL DEFAULT now(),
    -- An agent name identifies an agent within a tenant, like a flow name.
    UNIQUE (tenant_id, name)
);

ALTER TABLE nexus_agents ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_agents FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_agents_tenant_isolation ON nexus_agents
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_agents TO nexus_runtime;

CREATE TABLE nexus_agent_sessions (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   text NOT NULL,
    -- The agent this session runs against. Cascade-deleted with its agent so a
    -- removed agent leaves no orphan sessions.
    agent_id    uuid NOT NULL REFERENCES nexus_agents(id) ON DELETE CASCADE,
    -- Lifecycle: 'pending' (created, not yet run), 'running' (streaming now),
    -- 'completed' (finished), 'failed' (errored), 'cancelled'. Plain text rather
    -- than an enum so a new state never needs a migration.
    status      text NOT NULL DEFAULT 'pending',
    -- The accumulated message transcript as a jsonb array of {role, content}
    -- objects. Appended to as the session progresses; the durable record of the
    -- conversation, independent of any in-memory stream.
    transcript  jsonb NOT NULL DEFAULT '[]'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX nexus_agent_sessions_agent_idx
    ON nexus_agent_sessions (agent_id, created_at DESC);

ALTER TABLE nexus_agent_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_agent_sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_agent_sessions_tenant_isolation ON nexus_agent_sessions
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_agent_sessions TO nexus_runtime;
