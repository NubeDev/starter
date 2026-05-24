# Scope — rubix-agent-runtime

The authoritative design lives at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
§"Success criterion" + starter's
[`/home/user/code/rust/starter/DOCS/agent/SCOPE.md`](/home/user/code/rust/starter/DOCS/agent/SCOPE.md)
(R2 = `AiRunner` only LLM seam, R-agent-1 = the `ai-agent` node kind
is the agent primitive). Latest landed state is the most recent
session handoff under
[`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/).
Where this disagrees with either source, **the source wins** — fix
this file rather than diverge.

## Goal

Make the rubix MCP demo a **real LLM reasoning loop** instead of
the hand-rolled imposter currently in
[`rubix/crates/rubix-agent/src/boot/mcp.rs`](../../../rubix/crates/rubix-agent/src/boot/mcp.rs)
(~250 lines of `FlowBody::new()` + a fake `com.rubix.diag-render`
`NodeBehavior` that returns hardcoded Spanish strings).

After this job, calling `com.rubix.scheduled-system-check` over
MCP genuinely:

1. parses the bundled `scheduled-system-check.yaml` into a real
   `FlowBody`,
2. resolves its root `ai-agent` node against a real `NodeBehavior`
   that runs Claude CLI via `starter_ai::runners::claude::ClaudeRunner`,
3. dispatches the `rubix.system.disk` tool from the response,
4. returns a real `Diagnostic` built from the real probe output.

All five bundled goal flows (`dashboard-assistant`, `user-admin`,
`flow-programmer`, `clickhouse-ruler`, `weekly-report`) register
alongside `scheduled-system-check` and surface as MCP tools.

Three blocks, **all merged in one PR** against
`codeless/rubix-agent-runtime`, three REVIEW gates between them
inside the codeless job. Atomic — Block A+C without B has no
runtime; Block B without A+C has no consumer.

## What is already landed (do not redo)

These are on master at `8d84235` or earlier. Re-doing them
creates conflicts.

| What | Where |
|---|---|
| Six thin-slice PRs (1–5) | merged via PRs #27 + #28 |
| `starter_ai::runners::claude::ClaudeRunner` (a real `AiRunner` impl) | `crates/starter-ai/src/runners/claude.rs:23` |
| `AiRunner` trait + `Registry` + `Provider::ClaudeCli` | `crates/starter-spi/src/ai/runner.rs:51`, `crates/starter-ai/src/registry/...` |
| `starter-flow` engine + `NodeKindRegistry` + `FlowRegistry` + `FlowAsTool` | landed pre-#27 (U3) |
| Bundled YAMLs for all six goals | `rubix/crates/rubix-flows/flows/*.yaml` |
| `include_dir!` bundle helper | `rubix/crates/rubix-flows/src/lib.rs:12` |
| `starter-mcp` HTTP + stdio transports + `_meta.acceptLanguage` task-local | landed pre-#27 (U1+U2) |
| `MessageBundle::render_diagnostic` with timezone-aware `Timestamp` | `crates/starter-i18n/src/...` |

## What is already answered (do not re-litigate)

| # | Question | Answer locked |
|---|---|---|
| Q1 | `ai-agent` node kind impl | `starter-flow-node-loop` (thin wrapper over `starter-ai-agent` per the "not always nodes, wrappers later" decision below). |
| Loop scope | Strict vs thin loop | **Thin**: single-turn LLM → tool dispatch → response. No `SessionStore` persistence (in-memory only). No cost-cap. No `Cancel` token observation (outer `tokio::time::timeout` is the bound). The deferred concerns get a `LONG-TERM.md` design doc in `starter-flow-node-loop` so the next extension job has a contract to land against. |
| Provider | Which `AiRunner` | `ClaudeRunner` via `starter_ai::runners::claude::ClaudeRunner`. Operator can swap to `AnthropicRunner` via config; no auto-fallback. |
| Cleanup | Delete hand-rolled flow? | Yes — Block A deletes the ~250 lines of hand-rolled `FlowBody::new()` + the fake `com.rubix.diag-render` `NodeBehavior` from `boot/mcp.rs`. |
| Layering | Two crates or one | **Two**. `starter-ai-agent` is the runner-agnostic primitive (directly callable from a CLI, a REST handler, a test — no flow engine required). `starter-flow-node-loop` is the thin `NodeBehavior` adapter. This is the "not always nodes, wrappers later" architecture: the loop is testable without the engine; other callers don't pay the flow tax. |
| PR shape | Three PRs or one | **One PR**, three blocks, three REVIEW gates internal to the codeless job. Atomic merge. |

Anything else requiring a real decision: `BLOCKED:` escape hatch
in §"When codeless gets stuck" below.

## The two-crate split — pre-drafted API sketch

The architectural commitment: **the agent loop is not flow-shaped;
it is wrapper-shaped**. The primitive lives in `starter-ai-agent`
and is directly callable from anywhere. `starter-flow-node-loop`
is a thin `NodeBehavior` wrapper.

### `starter-ai-agent` (the primitive)

```rust
//! Single-turn LLM loop with tool dispatch. The agent primitive.
//!
//! Directly callable from a REST handler, a CLI, a test, a flow
//! node, or any other consumer. Does NOT depend on starter-flow.
//!
//! Thin v0: one LLM call → if the response carries tool calls,
//! dispatch them against the supplied ToolRegistry → re-call the
//! LLM with the tool results → return the final reply. No
//! multi-turn session persistence; no cost-cap; no Cancel
//! observation. See LONG-TERM.md for the deferred concerns and
//! the upstream contract the next extension job extends against.

use std::sync::Arc;
use async_trait::async_trait;
use starter_spi::ai::{AiRunner, RunnerInput, RunResult, SessionId};
use starter_spi::tool::Tool;

/// Errors the loop can surface. Mapped to `Diagnostic` by callers
/// that need locale-aware rendering.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("ai runner failed: {0}")]
    Runner(#[from] starter_spi::ai::RunnerError),

    #[error("tool {tool_id} returned error: {message}")]
    Tool { tool_id: String, message: String },

    #[error("tool {tool_id} not in registry")]
    UnknownTool { tool_id: String },

    /// The LLM response was not parseable as a final answer or a
    /// tool-call request. Carries the raw text so the caller can
    /// log or surface it.
    #[error("unparseable response: {0}")]
    Unparseable(String),
}

/// The handful of tools the agent is allowed to dispatch this turn.
/// Caller is responsible for filtering by skill / authz.
pub struct ToolSet {
    inner: Vec<Arc<dyn Tool>>,
}

impl ToolSet {
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self { Self { inner: tools } }
    pub fn get(&self, id: &str) -> Option<&Arc<dyn Tool>> {
        self.inner.iter().find(|t| t.definition().name == id)
    }
    pub fn all(&self) -> &[Arc<dyn Tool>] { &self.inner }
}

/// One agent turn. Owned by the caller — created per invocation,
/// dropped when done.
pub struct AgentLoop {
    runner: Arc<dyn AiRunner>,
    tools: ToolSet,
}

impl AgentLoop {
    pub fn new(runner: Arc<dyn AiRunner>, tools: ToolSet) -> Self {
        Self { runner, tools }
    }

    /// Drive one user prompt to a final reply. Synchronous from
    /// the caller's POV (one `.await`). Internally: one LLM call
    /// → optional tool dispatch loop → one final LLM call → return.
    ///
    /// `session_id` is opaque — the runner uses it for its own
    /// continuity, but this v0 does not persist anything across
    /// calls.
    pub async fn run(
        &self,
        prompt: &str,
        session_id: SessionId,
    ) -> Result<String, AgentError> {
        // Body sketch (codeless implements):
        //
        // 1. Build RunnerInput from prompt + self.tools' definitions.
        // 2. self.runner.run(...) → RunResult.
        // 3. Inspect RunResult.tool_calls (if any):
        //    for each ToolCall:
        //      let tool = self.tools.get(&tc.id).ok_or(UnknownTool)?;
        //      let result = tool.invoke(tc.args).await.map_err(|e| Tool { ... })?;
        //      collect (tc.id, result) for the next prompt turn
        // 4. If tool calls happened: build a follow-up RunnerInput
        //    carrying the tool results, re-call runner, return.
        // 5. If no tool calls: return the runner's reply text.
        //
        // Cancel: outer `tokio::time::timeout(d, agent.run(...))` is
        // the bound for v0; the LONG-TERM doc covers cooperative
        // cancellation via the existing `starter_spi::ai::Cancel`.
        todo!("codeless: implement per the sketch above")
    }
}
```

Crate `Cargo.toml`:
```toml
[package]
name = "starter-ai-agent"
description = "starter — runner-agnostic single-turn agent loop. Directly callable from anywhere; thin wrapper crates adapt to flow nodes / MCP tools / etc."
# version, edition, etc per workspace.

[dependencies]
starter-spi = { workspace = true, features = ["ai", "tool"] }  # AiRunner + Tool
async-trait = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

Crate layout (verb-per-file per FILE-LAYOUT):
```
crates/starter-ai-agent/
├── Cargo.toml
├── LONG-TERM.md             ← deferred concerns: sessions, cost-cap,
│                              multi-turn, cancellation, cost tracking
├── src/
│   ├── lib.rs               ← barrel + crate doc
│   ├── error.rs             ← AgentError
│   ├── tool_set.rs          ← ToolSet
│   ├── loop.rs              ← AgentLoop + run()
│   └── prompt.rs            ← prompt + tool-call serialisation
└── tests/
    ├── single_turn_no_tools_test.rs       ← runner replies directly
    ├── single_turn_with_tools_test.rs     ← runner asks for one tool call
    └── unknown_tool_test.rs               ← runner asks for a tool not in the set
```

Tests use a `MockAiRunner` (not the real Claude CLI) that returns
canned `RunResult`s. No live LLM in CI.

### `starter-flow-node-loop` (the thin wrapper)

```rust
//! NodeBehavior wrapper exposing AgentLoop as the `ai-agent`
//! node kind for starter-flow.
//!
//! Reads the prompt from the input slot, picks the allowed tool
//! set from the node's config slot, runs the loop, writes the
//! reply to the output slot. Per starter DOCS/agent/SCOPE.md
//! R-agent-1.

use std::sync::Arc;
use async_trait::async_trait;
use starter_ai_agent::{AgentLoop, ToolSet};
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeError, SlotMap};
use starter_spi::ai::AiRunner;
use starter_spi::tool::Tool;

/// The `ai-agent` node kind. Constructed once at boot with an
/// `AiRunner` + the host's `ToolRegistry`; cloned per invocation
/// (cheap — internal state is Arc).
pub struct AiAgentNode {
    runner: Arc<dyn AiRunner>,
    tools: Vec<Arc<dyn Tool>>,
}

impl AiAgentNode {
    pub fn new(runner: Arc<dyn AiRunner>, tools: Vec<Arc<dyn Tool>>) -> Self {
        Self { runner, tools }
    }

    /// Reverse-DNS id used by starter-flow's registry. Public so
    /// rubix-agent can reference it from boot::mcp.
    pub const KIND_ID: &'static str = "ai-agent";
}

#[async_trait]
impl NodeBehavior for AiAgentNode {
    async fn invoke(
        &self,
        ctx: &NodeCtx,
        input: SlotMap,
    ) -> Result<SlotMap, NodeError> {
        // Body sketch (codeless implements):
        //
        // 1. Read "prompt" string from input slot. NodeError if missing.
        // 2. Read "skill_hint" / "allowed_tools" from config slot
        //    (optional). If allowed_tools is set, filter self.tools
        //    by id. Otherwise pass all.
        // 3. Build a fresh AgentLoop with the filtered ToolSet and
        //    self.runner.clone(). session_id from ctx.run_id.
        // 4. .run(&prompt).await — map AgentError → NodeError.
        // 5. Write the reply text into the "out" slot of the
        //    returned SlotMap. Return.
        todo!("codeless: implement per the sketch above")
    }
}
```

Crate `Cargo.toml`:
```toml
[package]
name = "starter-flow-node-loop"
description = "starter — the `ai-agent` node kind. Thin NodeBehavior wrapper over starter-ai-agent. Per DOCS/agent/SCOPE.md R-agent-1."

[dependencies]
starter-ai-agent  = { workspace = true }
starter-flow-spi  = { workspace = true }
starter-spi       = { workspace = true, features = ["ai", "tool"] }
async-trait       = { workspace = true }
thiserror         = { workspace = true }
```

Crate layout:
```
crates/starter-flow-node-loop/
├── Cargo.toml
├── src/
│   ├── lib.rs               ← barrel + crate doc
│   └── node.rs              ← AiAgentNode
└── tests/
    └── invoke_test.rs        ← in-memory engine round-trip with MockAiRunner
```

## In scope (three blocks, all merged in one PR)

### Block A — YAML loader + register all six flows in rubix

In `rubix/`:

1. **New `rubix-flows/src/load.rs`** parsing each bundled YAML →
   typed `FlowBody`. Uses serde + `starter_flow::definition::body::FlowBody`
   constructors. Returns
   `fn load_all() -> Result<Vec<(FlowId, FlowRevisionId, FlowBody)>, LoadError>`.
   The loader resolves `ai-agent` node references to the
   `KIND_ID` constant from `starter-flow-node-loop` (which Block
   B introduces — so during Block A development the import points
   at a placeholder until Block B's crate exists).
2. **Rewrite `boot::mcp::build_mcp_surface`** to call
   `rubix_flows::load_all()` and register each flow via
   `FlowRegistry::register`. All six flows produce one MCP tool
   each via `FlowAsTool::from_registry`.
3. **Delete the hand-rolled flow body** in `boot/mcp.rs` (the
   `FlowBody::new()` chain that constructs
   `com.rubix.scheduled-system-check` by hand).
4. **Delete the fake `com.rubix.diag-render` `NodeBehavior`** and
   its `NodeKindRegistry::register` call. After deletion, `grep
   -rn 'diag-render\|diag_render' rubix/` returns empty.
5. **Update `docs/design/flows/README.md`** from placeholder /
   stale wording to present-tense: the loader is the contract;
   all six bundled YAMLs become MCP tools; the hand-rolled flow
   is gone.

### Block B — `starter-ai-agent` + `starter-flow-node-loop` upstream

In `starter/`:

1. **New crate `crates/starter-ai-agent/`** per the sketch above.
   Implement `AgentLoop::run`. Three unit tests pass against a
   `MockAiRunner`.
2. **New crate `crates/starter-flow-node-loop/`** per the sketch
   above. Implement `AiAgentNode::invoke`. One integration test
   spins up a tiny `starter-flow` engine, registers the kind,
   fires a flow whose root is `ai-agent`, asserts the reply makes
   it to the output slot.
3. **`crates/starter-ai-agent/LONG-TERM.md`** — design doc
   covering the deferred concerns:
   - **Multi-turn session persistence** via `SessionStore`
     (matches starter `DOCS/agent/SCOPE.md` R-agent-2). Names
     the future API surface: `AgentLoop::with_session_store(...)`
     and the per-turn checkpoint blob shape.
   - **Cost cap** (`AgentLoop::with_cost_cap(usd)`), enforced
     between turns; signals via a new `AgentError::CostCapHit`.
   - **Cooperative cancellation** via `starter_spi::ai::Cancel`
     observed inside the loop; signal via
     `AgentError::Cancelled`.
   - **Tool-call streaming** — today the loop only returns the
     final reply; future shape returns a stream of
     `AgentEvent::ToolStart` / `ToolComplete` / `Thinking` to
     match `rubix-spi`'s SSE taxonomy.
   - **Skill enforcement** — today the caller filters
     `allowed_tools` upfront; the LONG-TERM contract makes
     skill resolution a first-class loop concern.
4. **Workspace `Cargo.toml`** entries added for both new crates.
5. **Update starter's `docs/design/starter-changes/` ledger** to
   mark `starter-flow-node-loop` as **landed (in-tree)** and add
   a new entry for `starter-ai-agent` as the primitive layer.

### Block C — Wire AiRunner + register node kind in rubix-agent

In `rubix/crates/rubix-agent/`:

1. **New `boot/ai.rs`** verb file returning
   `Arc<dyn AiRunner>`. Default: `ClaudeRunner` via
   `starter_ai::runners::claude::ClaudeRunner`. Config knob in
   `AgentConfig` (from `boot/config.rs`) lets the operator pick
   `Provider::Anthropic` instead.
2. **Extend `boot::mcp::build_mcp_surface`** (from Block A) to:
   - construct an `AiAgentNode` via
     `AiAgentNode::new(runner, tool_registry.snapshot())`,
   - register it under `AiAgentNode::KIND_ID` (`"ai-agent"`) in
     the `NodeKindRegistry`.
3. **The bundled flows now actually fire.** A MCP `tools/call`
   for `com.rubix.scheduled-system-check` runs through the real
   loop, the LLM gets the disk-tool definition, picks it,
   dispatches it, and the renderer returns a real localised
   Diagnostic with the real disk number.
4. **Update `docs/design/agent/README.md`** with the runtime
   wiring: `ClaudeRunner` → `AgentLoop` →
   `AiAgentNode` → bundled flow → MCP.
5. **Update `docs/design/ai-providers/README.md`** with the
   provider-selection contract (default ClaudeCli; config knob
   for Anthropic REST).

## Out of scope (explicit carve-outs)

- **Multi-turn agent sessions.** `LONG-TERM.md` in
  `starter-ai-agent` is the contract for the next job that
  extends the loop. Not in this one.
- **Cost cap enforcement** beyond a hard outer
  `tokio::time::timeout`. Deferred to LONG-TERM.
- **Cooperative cancellation** via the existing `Cancel` trait.
  Deferred to LONG-TERM.
- **Tool-call streaming** through `AgentEvent::ToolStart` /
  `ToolComplete`. Deferred — the loop today returns only the
  final reply text. The R13 event taxonomy stays; this job just
  doesn't emit the agent-internal events yet.
- **Skill enforcement.** The caller (rubix's
  `boot::mcp::build_mcp_surface`) does the `allowed_tools`
  filtering today. Skill-driven filtering is LONG-TERM.
- **OAuth, gRPC, dashboards, flow programmer tools, analytics
  reports, user-admin tools, extension contribution.** Per the
  thin-slice non-goals.
- **Tool broadening** (the other 25 verb stubs). Comes after this
  job: a real runtime makes new tools instantly useful.
- **Promotion of the hardcoded `disk_used > 90` insights rule
  to `rule.rhai`.** T4 still locked.
- **`AnthropicRunner` testing.** Block C ships the config knob;
  actually testing the REST provider end-to-end is deferred to
  the operator (the demo path is ClaudeCli).
- **Codeless's existing `Runner` trait migration to consume
  `starter-ai-agent`.** That's a Codeless-side change; out of
  scope here.
- **Touching `rubix-old/`.** Read for archaeological context
  only; never copy code.

## Acceptance — when each block is "done"

### Block A — YAML loader
- `cargo build -p rubix-flows -p rubix-agent` green.
- `grep -rn 'diag-render\|diag_render\|FlowBody::new' rubix/crates/rubix-agent/src/` returns empty (hand-rolled flow + fake kind are gone).
- A new unit test in `rubix-flows/src/load.rs` (`#[cfg(test)] mod tests`) parses each of the six bundled YAMLs into a `FlowBody`. Asserts `flow_id`, root node id, root node kind = `"ai-agent"`.
- `boot::mcp::build_mcp_surface` registers all six flows; the startup log line shows `mcp_tools=6` (was 1).
- `docs/design/flows/README.md` is present-tense; no "stub", no "placeholder", no "PR 3".
- `./rubix/scripts/lint-doc-refs.sh` clean.
- **Note**: until Block B lands its crate, Block A's `load.rs` references `starter_flow_node_loop::AiAgentNode::KIND_ID` (or a string literal `"ai-agent"`) — either way, attempting to *invoke* the registered flows fails because the node kind has no behaviour yet. That's expected during the A-only state; Block C wires it.

### Block B — `starter-ai-agent` + `starter-flow-node-loop`
- `cargo build -p starter-ai-agent -p starter-flow-node-loop` green.
- `cargo test -p starter-ai-agent` green: three unit tests
  (single-turn no tools / single-turn with tools / unknown tool)
  pass against `MockAiRunner`.
- `cargo test -p starter-flow-node-loop` green: one integration
  test fires a flow through an in-memory engine and asserts the
  reply lands in the output slot.
- `cargo clippy --workspace -- -D warnings` clean (touch starter-spi
  features if `tool` isn't already exposed).
- `starter/crates/starter-ai-agent/LONG-TERM.md` exists with all
  five deferred concerns sectioned out.
- The workspace `Cargo.toml` lists both new crates in
  `[workspace.members]` and `[workspace.dependencies]`.
- The starter-changes ledger in `rubix/docs/design/starter-changes/README.md`
  has the matching entries marked landed.

### Block C — Wire AiRunner + register node kind
- `cargo build -p rubix-agent` green.
- `cargo test -p rubix-agent --test mcp_disk_test` green —
  the **same** test that today passes against the hand-rolled
  fake now passes against the real loop. (May need a recorded-LLM
  fixture; if so, record one and check it in. **No live LLM in
  CI.**)
- `cargo test -p rubix-agent --test mcp_stdio_test` green for
  both `en-US` and `es-AR` `_meta.acceptLanguage` values, again
  against the real loop.
- A `tools/list` against the MCP surface shows six MCP tools
  (was one).
- `docs/design/agent/README.md` describes the real runtime path
  present-tense.
- `docs/design/ai-providers/README.md` documents the config
  knob for runner selection.
- `./rubix/scripts/lint-doc-refs.sh` clean.

## Hard rules (subset that bites this job)

All from rubix `HOW-TO-CODE.md`, `FILE-LAYOUT.md`, `SCOPE.md`.

- **One verb per file**, ≤400 lines hard, ~100 typical. `mod.rs`
  is a barrel only.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only.
  `./rubix/scripts/lint-doc-refs.sh` enforces; run it before
  closing a stage.
- **No phasing markers** in code.
- **`Done`-doc handover paths must be listed individually** — no
  shell brace expansion, no globs, no leading `./`. The
  runtime's diff-verify pre-check is strict; see SCOPE.md §"Hard
  rules" and the upstream bug at
  `/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`.
- **No live LLM in CI.** Tests use `MockAiRunner` (a new test
  helper in `starter-ai-agent::testing`) or recorded fixtures
  from `starter-server::testing`.
- **No `clickhouse` direct dep on rubix crates.** Pull
  transitively. (Unchanged from prior jobs.)
- **Tool outputs are `Diagnostic` + structured data.** The
  agent loop's final reply is plain text, but any tool the loop
  dispatches returns a `Diagnostic`.

## When codeless gets stuck

Codeless cannot ask the human. So the escape hatch is:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever code does compile.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried and why it didn't match
   the spec.
4. Move to the next block only if it does not depend on the
   blocked one. Block A → B → C is a hard dependency chain;
   blocked A blocks the whole job.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Starter agent SCOPE (authoritative for R-agent-1, R-agent-2,
  R2, R4):
  [`/home/user/code/rust/starter/DOCS/agent/SCOPE.md`](/home/user/code/rust/starter/DOCS/agent/SCOPE.md)
- Current handoff: latest under
  [`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/).
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
- **Exemplars to copy religiously:**
  - **`AiRunner` trait shape**: `crates/starter-spi/src/ai/runner.rs:51` — `Send + Sync + 'static`, async-trait, `run(input, session_id, on_event, cancel)` returning `Result<RunResult, RunnerError>`.
  - **`ClaudeRunner` impl**: `crates/starter-ai/src/runners/claude.rs:23` — this is the runner Block C wires in.
  - **Codeless's `Runner` trait** (loop-shape reference, not the same trait):
    `/home/user/code/rust/codeless-workspace/codeless/crates/codeless-runtime/src/runner.rs:58`.
  - **Existing rubix hand-rolled flow** (the thing Block A deletes):
    `rubix/crates/rubix-agent/src/boot/mcp.rs:82-356`.
  - **`FlowAsTool::from_registry`** (U3): `crates/starter-flow-surfaces/src/...`.
  - **`include_dir!` bundle pattern**:
    `rubix/crates/rubix-flows/src/lib.rs`.
  - **Verb-per-file Cargo.toml + crate layout**: any existing
    `crates/starter-*` crate.
