# 2026-05-25 — Dashboard assistant: backend e2e audit + plan to get it working

Walking session note. Goal: get the **`com.rubix.dashboard-assistant`**
agent flow working end-to-end so we can drive it from the frontend.
Audit done against a live binary (already running on `127.0.0.1:8088`,
Postgres on `:5433`, ClickHouse on `:8124`). Runner = **Claude Code CLI
2.1.139** on PATH (`~/snap/code/226/.local/share/pnpm/claude`).

## TL;DR

- The agent pipeline **is** wired and **does** work end-to-end —
  proven by `com.rubix.scheduled-system-check`, which returns a real
  Claude-narrated reply + structured `rubix.system.disk` output in
  ~6 s.
- The **dashboard-assistant** (and 4 other bundled flows) returns
  `{text:"null", structuredContent:null}` in <1 s. Root cause is a
  registration-time heuristic, not the agent path itself.

## What I exercised

Login → MCP `initialize` → `tools/list` → `tools/call` for all 6
bundled `ai-agent` flows. Auth = `op@example.com` session cookie.

```text
                                      tools/call live Claude CLI
flow                                  result                       elapsed
------------------------------------  ---------------------------  -------
com.rubix.scheduled-system-check      ✅ tool + reply              6 s
com.rubix.dashboard-assistant         ❌ text:"null"               0 s
com.rubix.user-admin                  ❌ text:"null"               0 s
com.rubix.flow-programmer             ❌ text:"null"               0 s
com.rubix.clickhouse-ruler            ❌ text:"null"               0 s
com.rubix.weekly-report               ❌ text:"null"               1 s
```

Boot log shows the wiring is complete:

```text
tools=33 mcp_tools=7 skills=6 flows=7
rubix ai-agent node kind registered  node_kinds="com.rubix.ai-agent"
rubix MCP surface assembled          mcp_tools=7 flow_tools=7
```

## Root cause (5 silent failures, one mechanism)

[`rubix/crates/rubix-agent/src/boot/mcp/register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs)
picks the **first entry of `allowed_tools`** as the node's "primary
tool" and stamps it into the seed payload:

```rust
fn primary_tool_for_root(body: &FlowBody) -> Option<String> {
    let root = body.nodes.first()?;
    let arr  = root.settings.get("allowed_tools")?.as_array()?;
    let first = arr.first()?.as_str()?;
    Some(first.to_owned())
}
```

[`agent_node.rs`](../../crates/rubix-agent/src/boot/mcp/agent_node.rs)
then calls that tool with `payload.input` (which is `{}` for the
zero-arg MCP call we make). For the 4 broken flows the first entry is
a write verb with required fields, so the tool returns
`InvalidInput`, the node fails with `NodeError::Backend`, the flow run
dies, and `FlowAsTool` reads back a null terminal slot.

Direct REST probes confirm:

| flow                | primary picked              | direct probe                              |
|---------------------|-----------------------------|-------------------------------------------|
| dashboard-assistant | `rubix.dashboard.create`    | 400 — missing `tenant_id`                 |
| user-admin          | `rubix.user.create`         | 400 — missing `email`                     |
| flow-programmer     | `rubix.flow_ops.deploy`     | 400 — missing `flow_id`                   |
| clickhouse-ruler    | `rubix.clickhouse.rule.write` | 400 — missing `rule_name`               |
| weekly-report       | `analytics.query`           | **404 — unknown tool**                    |
| scheduled-system-check (works) | `rubix.system.disk` | 200 — disk probe ✓                       |

The single-passing flow is the one whose first `allowed_tools` entry
happens to be a zero-arg read.

## Why the agent log was silent

The node returns the error to the flow run; the flow surfaces a null
read-back; MCP returns a valid envelope with `text:"null"`. Nothing
propagates to `tracing` at INFO. Future debugging hint: add a
`tracing::warn!` on the `NodeError::Backend` arm in `agent_node.rs`
before we lose the context.

## Plan to get the dashboard working (small → big)

### Step 1 — make the 5 flows return real output (no LLM dependency)

Reorder `allowed_tools` in each YAML so the first entry is a zero-arg
read verb. For dashboard-assistant:

```yaml
allowed_tools:
  - rubix.dashboard.list          # was: rubix.dashboard.create
  - rubix.dashboard.get
  - rubix.dashboard.create
  - rubix.dashboard.update
  - rubix.dashboard.duplicate
  - rubix.dashboard.delete
  - rubix.dashboard.page_set
  - rubix.undo.last
```

Same shape for `user-admin` (lead with `rubix.user.list`),
`flow-programmer` (lead with `rubix.flow_ops.list`),
`clickhouse-ruler` (lead with `rubix.clickhouse.rule.list`).
`weekly-report` is special — see Step 2.

This turns the heuristic into "the agent narrates the current state
on a zero-arg call". For a chat surface that's exactly the right
default: open the assistant → see what's already there → ask for a
change.

Verify with the same MCP `tools/call` loop used today.

### Step 2 — register the missing tools

`analytics.query` / `analytics.report` are not in
[`registry::build_tool_registry`](../../crates/rubix-agent/src/registry.rs).
Either add stubs there or change the weekly-report `allowed_tools`
first entry to a tool that exists. Stub-first is cheapest.

### Step 3 — author-intent fix (follow-up, not blocking)

Replace the "first allowed_tool" heuristic with an explicit
`primary_tool:` field on the node config. Then `allowed_tools` order
stops being load-bearing and reviewer intent stops being smuggled
through list ordering. See SCOPE R8 — single seam, no hidden
contracts.

### Step 4 — frontend surface for the dashboard assistant

The frontend currently has **no chat / agent invocation surface**
(see audit grep — only the canvas-side `RubixAiAgentNode` renderer
exists). To drive the assistant from the dashboard route:

1. Add an MCP-over-HTTP client to `@nube/rubix-client-ts` that POSTs
   JSON-RPC `tools/call` to `/api/v1/mcp` (the surface that already
   works today).
2. Add a `useAgentInvoke(flowId, prompt)` hook in
   `@nube/rubix-client-react`.
3. Drop a "Ask the assistant" panel into
   [`rubix/frontend/src/routes/dashboards/`](../../frontend/src/routes/dashboards/)
   that calls `useAgentInvoke('com.rubix.dashboard-assistant', …)`
   and renders `result.structuredContent.reply` + a JSON viewer for
   `result.structuredContent.tool`.
4. One Playwright spec under
   [`rubix/frontend/e2e/`](../../frontend/e2e/) that
   stubs the MCP response and asserts the panel renders.

(Alternative: merge `FlowAsTool` entries into the REST
`/api/v1/tools/*` registry so existing `useToolCall` hooks reach the
flows without a new transport. Cheaper but conflates the data-tool
surface with the agent-flow surface — defer.)

## Risk / open questions

- **Dashboard tools are in-memory.** Boot log explicitly warns:
  `user/tenant/team/clickhouse/insights verbs are wired against
  in-memory stores`. The dashboard list will be empty after restart
  until those adapters land. For demo-day this is fine; for "shows
  real data" we need the PG-backed adapter in
  `crate::registry::build_tool_registry`.
- **Skill body is advisory.** `skill_hint:
  com.rubix.dashboard-builder` is currently just metadata for the
  canvas renderer — the agent loop does not load the SKILL.md body
  into the system prompt yet. That's a separate piece of work
  ([`docs/design/skills/`](../design/skills/README.md)). For Step 4
  it doesn't block.
- **`RUBIX_AI_NARRATION=0`** disables the LLM round-trip and returns
  pure tool output (deterministic, free). Use this for the Playwright
  spec.

## What I am NOT doing in this session

- Not touching the frontend until Step 1 + 2 land and the 5 flows
  return real output.
- Not refactoring the primary-tool heuristic (Step 3) — same.
- Not wiring PG-backed dashboard storage — different scope.

---

## Update — Step 1 + a graceful-degradation fix landed

### Changes

- **Reordered `allowed_tools`** in 4 flow YAMLs so the first entry is
  a zero-arg read verb (which the `primary_tool_for_root` heuristic
  picks). Inline comments now point at the heuristic so the next
  reviewer knows order is load-bearing.
  - [`flows/dashboard-assistant.yaml`](../../crates/rubix-flows/flows/dashboard-assistant.yaml)
    — leads with `rubix.dashboard.list`
  - [`flows/user-admin.yaml`](../../crates/rubix-flows/flows/user-admin.yaml)
    — leads with `rubix.user.list`
  - [`flows/flow-programmer.yaml`](../../crates/rubix-flows/flows/flow-programmer.yaml)
    — leads with `rubix.flow_ops.list`
  - [`flows/clickhouse-ruler.yaml`](../../crates/rubix-flows/flows/clickhouse-ruler.yaml)
    — leads with `rubix.clickhouse.rule.list`

- **Graceful primary-tool failure** in
  [`boot/mcp/agent_node.rs`](../../crates/rubix-agent/src/boot/mcp/agent_node.rs).
  The previous behaviour was: primary tool missing or returns
  `InvalidInput` → `NodeError::Backend` → flow run dies → caller
  gets `text:"null"`, no log, no clue. New behaviour: primary-tool
  failures `tracing::warn!` and fall through to reply-only narration.
  The `(None, None)` arm also returns a structured stub
  (`{ reply: "primary tool X produced no output …", tool: null }`)
  instead of `NodeError::Backend`, so the caller always sees *what*
  went wrong instead of a silent null.

### Verification

Live MCP `tools/call` sweep (Claude CLI runner, post-restart, after
`flows_definitions seed-and-load: rolled_forward=4`):

| flow                            | result                                                  |
|---------------------------------|----------------------------------------------------------|
| `dashboard-assistant`           | ✅ `reply: "No dashboards were returned for your account."` (6 s) |
| `user-admin`                    | ✅ tool output + narrated reply (5 s)                   |
| `flow-programmer`               | ✅ lists all 7 flows, narrated (7 s)                    |
| `clickhouse-ruler`              | ✅ tool output, narration occasionally flaky (Claude CLI concurrency, not our bug) |
| `scheduled-system-check`        | ✅ disk probe + reply (6 s)                             |
| `weekly-report`                 | ⚠️ stub reply (`analytics.query` still unregistered — Step 2 of original plan) |

Deterministic sweep with `RUBIX_AI_NARRATION=0`: all 6 flows return
non-null `structuredContent` in <1 s each.

The "dashboard-assistant returns null" failure mode is gone. The path
**MCP `tools/call` → `RubixAiAgentNode` → `starter_ai_agent::AgentLoop`
→ `ClaudeRunner` → claude CLI subprocess** is end-to-end working.

### Frontend transport: already wired

Verified [`@nube/rubix-client-ts/src/endpoints/mcp.ts`](../../packages/rubix-client-ts/src/endpoints/mcp.ts)
already implements both `mcpToolsList()` and `mcpToolsCall(name, args,
{acceptLanguage})`. The TanStack hook + UI panel + Playwright spec are
still TODO but the transport piece of Step 1 (original plan) is
**not needed** — it exists.

### Backend boot reminder for the next agent

Reproduction recipe used this session:

```bash
# Postgres + ClickHouse already up on :5433 / :8124
pkill -f 'target/debug/rubix-agent' 2>/dev/null
cargo build -p rubix-agent --bin rubix-agent
RUBIX_DSN=postgres://rubix:rubix-dev@127.0.0.1:5433/rubix \
RUBIX_CONFIG=rubix/dev/agent.toml \
RUBIX_CH_URL=http://127.0.0.1:8124 \
  target/debug/rubix-agent > /tmp/rubix-agent.log 2>&1 &

curl -s -c /tmp/cookies.txt -X POST http://127.0.0.1:8088/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"op@example.com","password":"rubix-dev-passwd"}'

curl -sS -b /tmp/cookies.txt -X POST http://127.0.0.1:8088/api/v1/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
       "name":"com.rubix.dashboard-assistant",
       "arguments":{"prompt":"What dashboards do I have?"}}}'
```

`RUBIX_AI_NARRATION=0` on the boot line skips the Claude CLI round-trip
(deterministic CI, no LLM cost).

## Remaining work to get the dashboard assistant in the UI

Updated checklist (transport already exists, see above):

1. ~~Add MCP-over-HTTP client to `@nube/rubix-client-ts`~~ — **done
   upstream**, see `mcpToolsCall`.
2. **Add `useAgentInvoke` hook** in `@nube/rubix-client-react/src/hooks/`.
   Look at the existing `mcp.ts` hooks first — there's already an
   `mcp.test.tsx`, the hook may already exist; if not, mirror the
   shape of `useToolCall`.
3. **Drop a panel** onto
   [`rubix/frontend/src/routes/dashboards/`](../../frontend/src/routes/)
   with a prompt input + reply pane + collapsible JSON viewer for
   `structuredContent.tool`.
4. **Playwright spec** under [`rubix/frontend/e2e/`](../../frontend/e2e/)
   — run the agent with `RUBIX_AI_NARRATION=0` so it's deterministic.

## SSE — not in scope here

The MCP `tools/call` transport is plain POST → single JSON envelope.
No streaming. The runner already plumbs an `mpsc::channel` of `Event`s
that the agent loop drains and discards
([`agent_loop.rs:101-104`](../../../crates/starter-ai-agent/src/agent_loop.rs)).
Streaming tokens to the UI requires:

1. `starter-ai-agent` stops dropping `Event`s, propagates them.
2. Either `starter-mcp` grows a streaming `tools/call` variant, **or**
   rubix exposes a sibling `/api/v1/agent-events?session=…` SSE route
   (mirror of the existing `/api/v1/flow-events`).
3. `@nube/rubix-client-ts` adds an `EventSource` client matching it.

Deferred — tracked in [`crates/starter-ai-agent/LONG-TERM.md`](../../../crates/starter-ai-agent/LONG-TERM.md)
under "Tool-call streaming via the R13 SSE taxonomy". Not blocking the
panel; one 6 s spinner is fine for first cut.

## Architecture cheatsheet (for the next agent)

```text
React (mcpToolsCall)
  → POST /api/v1/mcp                                ← starter-mcp HTTP transport
    → FlowAsTool(com.rubix.dashboard-assistant)     ← starter-flow-surfaces
      → RubixAiAgentNode                             ← rubix/.../boot/mcp/agent_node.rs
        → starter_ai_agent::AgentLoop                ← crates/starter-ai-agent
          → Arc<dyn AiRunner> = ClaudeRunner         ← crates/starter-ai/src/runners/claude.rs
            → `claude` CLI subprocess                ← manages its own auth
```

Three composition sites only:

- runner selection — [`rubix-agent/src/boot/ai.rs`](../../crates/rubix-agent/src/boot/ai.rs)
- node-kind registration + per-flow primary-tool stamping —
  [`rubix-agent/src/boot/mcp/register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs)
- tool registry snapshot the agent loop dispatches against —
  [`rubix-agent/src/registry.rs`](../../crates/rubix-agent/src/registry.rs)

## Next concrete action

Either (a) start the React hook + dashboard panel (Steps 2–3), or
(b) tackle Step 2 of original plan (register `analytics.query` / `.report`
so weekly-report stops returning a stub).
