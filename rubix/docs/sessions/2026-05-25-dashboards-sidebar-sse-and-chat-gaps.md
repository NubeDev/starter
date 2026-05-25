# 2026-05-25 — Live dashboard sidebar SSE landed, three store bugs fixed, chat-streaming audit

Walking session note. Follows
[`2026-05-25-dashboard-assistant-e2e.md`](2026-05-25-dashboard-assistant-e2e.md).
Goal this session: get the chat tab actually authoring dashboards
end-to-end, then expose new pages in the sidebar live. Both halves
landed; tool-calling-via-chat and ChatGPT-style streaming did not —
the bottom of this doc lays out exactly why, with file:line evidence,
and the corrected plan for next session.

## TL;DR

- **Fixed three store-split bugs** uncovered by an e2e of the
  dashboard-assistant flow. Chat can now create dashboards, and
  every surface (REST + MCP + SDUI) reads the same Postgres rows.
- **Shipped the live-sidebar SSE feature** per
  [`docs/scope/dashboards/09-live-sidebar-sse.md`](../scope/dashboards/09-live-sidebar-sse.md):
  `GET /api/v1/dashboards/events` + `useDashboardSidebar()` +
  dynamic NAV groups. Verified end-to-end (REST create → sidebar
  entry within ~1 s, no reload).
- **Untangled two myths** about what's still missing:
  1. Per-token streaming **is** built — every runner already pushes
     `EventKind::{Text,ToolUse,Done}` into `mpsc::Sender<Event>`.
     Rubix's chat path drops the receiver
     ([`agent_loop.rs:99`](../../../crates/starter-ai-agent/src/agent_loop.rs#L99) —
     `let (tx, _rx) = mpsc::channel(16);`).
  2. Claude-CLI tool dispatch **is** built — `CliCfg` has
     `mcp_url`/`mcp_token`/`mcp_config_path` and the runner already
     wires it. Rubix never sets them, so the model never sees the
     tools.
- **One thing genuinely missing:** the boot wires
  `RubixAiAgentNode` (using `AgentLoop` v0) instead of
  `starter-flow-nodes::AiAgentBehavior` (the real multi-turn loop
  that the rest of the workspace uses). Swap is the unlock for
  both chat streaming and chat-tool-calling.

## Part 1 — Three bugs that broke the chat→DB→SDUI loop

End-to-end exercise of the bundled `com.rubix.dashboard-assistant`
flow against a live agent surfaced three failures the smoke notes
did not catch because each surface had been tested in isolation:

### B1 — three-way store split

`rubix-agent` was constructing **three different**
`Arc<dyn DashboardStore>` instances:
- the REST tools registry (in [`registry.rs`](../../crates/rubix-agent/src/registry.rs))
- the MCP `tool_registry_snapshot` (in [`boot/mcp/register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs))
- the SDUI `PgPageProvider` (in [`boot/`](../../crates/rubix-agent/src/boot/))

All three were `InMemoryDashboardStore::new()` on the laptop path,
so a `POST /api/v1/tools/rubix.dashboard.create` landed in REST's
map; `/api/v1/ui/resolve` 404'd; the chat's `rubix.dashboard.list`
returned empty.

**Fix.** Two parts, both in
[`crates/rubix-agent/src/`](../../crates/rubix-agent/src/):
1. `registry.rs` now switches dashboard store on `pg_pool.as_ref()`:
   `PgDashboardStore::new(pool.clone())` when present, otherwise
   in-memory. Same shape already used for `flow_store`.
2. `boot/mcp/mod.rs` + `register.rs` gain a `shared_tools:
   Option<Vec<Arc<dyn Tool>>>` parameter threaded through
   `build_mcp_surface` → `build_tool_registry` → `build_flow_registry`.
   `main.rs` builds the tool registry **once**, then passes
   `Some(tools.clone())` to MCP so REST and MCP carry the same
   `Arc` instances (and therefore the same store).

### B2 — Claude CLI never gets `tools`

[`crates/starter-ai-agent/src/agent_loop.rs`](../../../crates/starter-ai-agent/src/agent_loop.rs)
flattens CLI prompts into one combined string and **omits**
`tools: self.tools.definitions()` for `Provider::Claude | Codex |
Copilot`. So even when MCP carries the catalogue, the model never
hears about it.

**Status this session: deferred.** See
[Part 3](#part-3--chat-streaming--tool-dispatch-via-chat-the-corrected-plan)
for the corrected fix path (we have everything we need; it's a
wiring job, not net-new code).

### B3 — session principal not propagated into seed payload

The seed adapter in
[`crates/rubix-agent/src/boot/mcp/register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs)
serialised the caller's MCP `arguments` straight into the flow's
seed slot. A prompt-only chat call (`{"prompt":"..."}`) therefore
hit `rubix.dashboard.create` with no `tenant_id` → 400 `missing
field tenant_id`.

**First attempt — wrong location.** I added the augment in
`boot/mcp/agent_node.rs`. It compiled, but
`starter_mcp::current_principal()` returned `None` at run time —
the principal task-local does **not** propagate across the
`tokio::spawn` that
[`FlowAsTool::invoke_with_cancel`](../../../crates/starter-flow-surfaces/src/lib.rs#L143)
uses to launch the engine run. Silent no-op in production.

**Real fix.** Moved enrichment into the **seed adapter**, which
runs synchronously on the request task *before* the engine
spawns. New `enrich_input_with_principal()` in
[`boot/mcp/register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs)
fills `tenant_id` (→ `DEFAULT_TENANT="system"` when the session
is unbound, matching the seed data), `owner_principal`, and
`created_by` from `starter_mcp::current_principal()` only when
the caller omitted them. Same constant is used in
`agent_node.rs` (defensive double-fallback) and in
`routes/dashboard_events.rs` (snapshot + delta filter).

### Verification

```bash
# REST → MCP → SDUI agreement
curl -b cookies … rubix.dashboard.create  → 201
curl -b cookies … rubix.dashboard.list    → [{disk-overview}, {new}]
curl -b cookies … /api/v1/ui/resolve      → renders the new page

# Chat with prompt only (was: missing tenant_id)
curl -d '{"name":"com.rubix.dashboard-assistant","arguments":{"prompt":"list my dashboards"}}'
→ "Found 2 dashboards: Disk overview and E2E Test Dashboard."  count=2
```

`cargo test -p rubix-agent --lib` → 53 passed.

### Lesson

Silent in-memory store duplication across surfaces is invisible
until something exercises the full read-write loop end-to-end.
And: **task-locals do not survive `tokio::spawn`** — read them on
the request task, snapshot, pass by value.

## Part 2 — Live sidebar over SSE

Spec: [`docs/scope/dashboards/09-live-sidebar-sse.md`](../scope/dashboards/09-live-sidebar-sse.md).

### What shipped

| Layer | File | Notes |
|---|---|---|
| SSE route | [`crates/rubix-agent/src/routes/dashboard_events.rs`](../../crates/rubix-agent/src/routes/dashboard_events.rs) | `GET /api/v1/dashboards/events`. First frame is `snapshot`; deltas are `created`/`updated`/`deleted`. Uses `PgListenTail` + `PgDashboardStore`. Anonymous → 401. 15 s keep-alive. 6 unit tests. |
| Client hook | [`packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts`](../../packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts) | Wraps `useEventStream` (same pattern as `flow-events.ts`). Idempotent reducer keyed by `page_id`; snapshot wins over in-flight deltas via a `generation` counter. 4 tests. |
| Dynamic NAV | [`frontend/src/lib/use-nav-groups.ts`](../../frontend/src/lib/use-nav-groups.ts) | Splits `NAV_GROUPS` into static + live Dashboards group at index 1. Empty-state CTA → `/chat`. Reconnect badge only on the first item to avoid a sea of dots. |
| i18n | en + es | `nav.group.dashboards`, `nav.item.createFirstDashboard`. |

### Bridge to the current change-log

`PgDashboardStore` does not (yet) emit `rubix.dashboard.page`
rows directly. The SSE handler therefore recognises two source
shapes in
[`change_to_event`](../../crates/rubix-agent/src/routes/dashboard_events.rs):
1. Future: a direct `rubix.dashboard.page` change row.
2. Today: the existing `tool.invoke` audit row for one of
   `rubix.dashboard.{create,update,delete}`, with `tenant_id` /
   `page_id` / `title` plucked from the redacted request body.

When a future PR adds a direct `ChangeRecorder` hook in
`PgDashboardStore`, the synthesis branch becomes a one-line
change; the wire format does not move.

### Live verification

```bash
# tab 1: open SSE
curl -sN -b cookies http://127.0.0.1:8088/api/v1/dashboards/events
data: {"kind":"snapshot","items":[…]}
# tab 2 (≈1 s later):
curl -X POST … rubix.dashboard.create → 201
# tab 1 immediately receives:
data: {"kind":"created","page_id":"dashboard.live-sse-demo","title":"Live SSE Demo","tenant_id":"system"}
```

In the browser, `chat.tsx` triggers a create → the sidebar grows
a new entry within the same animation frame.

## Part 3 — Chat streaming + tool-dispatch-via-chat: the corrected plan

After the user pushed back on "step 4 should already be there", I
re-read the actual code instead of trusting `LONG-TERM.md`. They
were right. Here is what's actually in the repo today.

### Streaming exists at the runner layer

[`crates/starter-spi/src/ai/runner.rs:47`](../../../crates/starter-spi/src/ai/runner.rs#L47):

```rust
pub type OnEvent = mpsc::Sender<Event>;
```

Every runner pushes `EventKind::{Connected, Text, ToolUse, Done,
Error}` into it as the stream lands. The Claude binary even
emits per-chunk text frames via `claude_wrapper::stream_query`
in [`runners/claude.rs:256-322`](../../../crates/starter-ai/src/runners/claude.rs#L256-L322).

The SSE-from-channel bridge is already shipped in
[`examples/flow-agent/src/rest.rs:519-545`](../../../examples/flow-agent/src/rest.rs#L519-L545):

```rust
let stream = s.ai.run_agent(&agent, body.input.text, history)?;
let sse = Sse::new(stream).keep_alive(...);
```

That is the pattern `POST /api/v1/chat/stream` needs to copy in
`rubix-agent`. Not new code — a port.

### Where rubix drops it on the floor

[`crates/starter-ai-agent/src/agent_loop.rs:99`](../../../crates/starter-ai-agent/src/agent_loop.rs#L99):

```rust
let (tx, _rx) = mpsc::channel(16);    // receiver dropped immediately
```

The MCP `tools/call` is also one-shot JSON-RPC — even if the
channel were drained, there is no transport carrying the deltas
to the browser.

The bigger `starter-flow-nodes::AiAgentBehavior` (in
[`crates/starter-flow-nodes/src/ai_agent.rs`](../../../crates/starter-flow-nodes/src/ai_agent.rs))
does the same drop at line 759 (`while rx.recv().await.is_some()
{}`), but it is the real multi-turn loop with proper CLI + REST
branches and skill plumbing. Rubix should use **this**, not the
v0 `AgentLoop`.

### Claude CLI tool dispatch — built, not wired

[`CliCfg`](../../../crates/starter-spi/src/ai/input.rs#L66-L75)
already has `mcp_url`, `mcp_token`, `mcp_config_path`. When set,
[`runners/claude.rs:118-145`](../../../crates/starter-ai/src/runners/claude.rs#L118-L145)
writes the MCP config the Claude binary needs, and the model can
call **our** tools through that bridge.

Rubix never populates these on the seed adapter, so the model
never sees the tool catalogue, so it just narrates.

### Skill body — read at boot, never injected

Boot logs `skills=6` because
[`rubix_skills::bundled()`](../../crates/rubix-skills/src/lib.rs)
parses all six SKILL.md files including
`dashboard-builder/SKILL.md`. The flow's `skill_hint:
com.rubix.dashboard-builder` reaches
[`ai_agent.rs:347-350`](../../../crates/starter-flow-nodes/src/ai_agent.rs#L347-L350),
which **warns** and drops it on the floor:

```rust
if let Some(hint) = cfg.skill_hint.as_ref() {
    warn!(skill_hint = %hint,
        "ai_agent: skill_hint override is no-op until starter-skills lands; using ctx.skill");
}
```

The body is never put into `system_prompt`.

### Plan (next session)

| Step | Where | Effect |
|---|---|---|
| 1. Swap `RubixAiAgentNode` → `starter-flow-nodes::AiAgentBehavior` | [`crates/rubix-agent/src/boot/mcp/agent_node.rs`](../../crates/rubix-agent/src/boot/mcp/agent_node.rs) + [`register.rs`](../../crates/rubix-agent/src/boot/mcp/register.rs) | Multi-turn loop, real tool path, event channel we can plug a sink into. |
| 2. Inject skill body | seed adapter in `register.rs` | `rubix_skills::bundled().get(skill_hint).body()` → `system_prompt`. Model finally has the dashboard-builder playbook. |
| 3. Set `CliCfg.mcp_url` + service token | seed adapter | Claude binary attaches as MCP client; the model can finally dispatch our tools. |
| 4. Add `POST /api/v1/chat/stream` SSE route | new file under [`crates/rubix-agent/src/routes/`](../../crates/rubix-agent/src/routes/) | Copy the pattern from `examples/flow-agent/src/rest.rs:519-545`. Spawn flow run with an event sink; forward `EventKind::Text` → `data: {"type":"text","delta":"…"}` frames; `ToolUse` → `data: {"type":"tool","name":"…"}`. |
| 5. Chat tab consumes SSE | [`frontend/src/routes/chat.tsx`](../../frontend/src/routes/chat.tsx) | Replace the `useToolCall` mutation with `useEventStream<ChatFrame>('/api/v1/chat/stream')`; append-as-you-go to the bubble. ChatGPT-shaped typing animation. |

Steps 1–3 make "make me an iot dashboard" actually create
something. Step 4–5 make it feel alive. Both halves should be an
afternoon each given how much of the substrate already exists.

## Files touched

```
crates/starter-flow-nodes/src/ai_agent.rs               # untouched, audited
crates/starter-ai/src/runners/claude.rs                 # untouched, audited

rubix/crates/rubix-agent/src/registry.rs                # B1: PgDashboardStore when pool present
rubix/crates/rubix-agent/src/boot/mcp/mod.rs            # B1: shared_tools threaded
rubix/crates/rubix-agent/src/boot/mcp/register.rs       # B1 + B3 + skill-body TODO
rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs  # B1: stdio caller updated
rubix/crates/rubix-agent/src/main.rs                    # B1: tool registry built before MCP
rubix/crates/rubix-agent/src/boot/mcp/agent_node.rs     # B3: defensive augment + DEFAULT_TENANT

rubix/crates/rubix-agent/src/routes/dashboard_events.rs # NEW — SSE sidebar
rubix/crates/rubix-agent/src/routes/mod.rs              # mount

rubix/packages/rubix-client-react/src/hooks/use-dashboard-sidebar.ts       # NEW
rubix/packages/rubix-client-react/src/hooks/use-dashboard-sidebar.test.tsx # NEW (4 tests)
rubix/packages/rubix-client-react/src/index.ts                              # re-export

rubix/frontend/src/lib/use-nav-groups.ts                # NEW dynamic NAV
rubix/frontend/src/components/nav/AppSidebar.tsx        # consume useNavGroups()
rubix/frontend/src/i18n/{en,es}.json                    # nav.group.dashboards, etc.

rubix/docs/scope/dashboards/09-live-sidebar-sse.md      # NEW scope doc
```

## Open items for next session

- Wire steps 1–3 above and rerun "make me an iot dashboard" end
  to end. Acceptance: a brand-new `page_id` arrives in
  `dashboards_definitions`, the sidebar grows an entry within
  ~1 s, the route renders the AI-authored page via SDUI.
- Then steps 4–5. Acceptance: chat bubble fills character by
  character; tool dispatches render as inline pills as they
  fire.
- Optional cleanup: have `PgDashboardStore` write
  `rubix.dashboard.page` change rows directly, then collapse the
  `tool.invoke` synthesis branch in `dashboard_events.rs`.
- Bootstrap-user gap: `op@example.com` has no `tenant_id`. The
  `DEFAULT_TENANT="system"` fallback covers single-tenant dev
  but masks a real multi-tenant bug. Either add a `--tenant`
  flag to `rubix-admin bootstrap-user` or default new users to a
  membership in the seeded `system` tenant.
