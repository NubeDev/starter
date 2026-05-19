# Scope — starter-flow-scaffold

> Source of truth: [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> in the starter repo. This file is the per-job brief the runner
> reads before every stage; intentionally short. When this file
> disagrees with the source-of-truth SCOPE, that doc wins — open an
> issue and update this file.

## Goal

Lay the **core dirs and files** for `starter-flow` per the flow
SCOPE — Phase 1 contracts in full (`starter-flow-spi`), plus empty
crate skeletons for `starter-flow` (engine), `starter-flow-nodes`
(built-in node kinds), and `starter-flow-surfaces` (FlowAsTool /
FlowAsService). The workspace builds end-to-end; **nothing
executes yet**. Doing this first is what makes the agent-vs-flow
sequencing question moot: both fold into the same node-graph
primitive, and Phase 4 of the flow SCOPE eventually subsumes the
agent SCOPE's R1 by collapsing the AI agent into one node kind.

## In scope

- **`starter-flow-spi`** — Phase 1 in full per the SCOPE's
  §"What lands in `starter-flow-spi`":
  - `node` module: `NodeBehavior` trait (async `invoke` +
    `on_lifecycle`, `&self` per R5), `NodeId` / `KindId`
    reverse-DNS newtypes (R10), `SlotRef`, `SlotValue` enum
    (Null / Bool / Int / Float / String / Bytes / Json) +
    `SlotMap`.
  - `graph` module: `GraphStore` trait (async `write_slot` /
    `read_slot` / `subscribe`), `WriteSlotOpts` carrying the
    `replay: bool` flag per R2's replay-does-not-re-fire-
    subscribers rule, `SubscribeOpts`, `SubscriptionStream`
    associated type.
  - `flow` module: `FlowId` / `FlowRevisionId` / `RunId`
    newtypes, `FlowEvent` enum (RunStarted / NodeStarted /
    NodeEmitted / NodeFailed / RunCompleted / RunFailed /
    RunCancelled), `FlowStore` / `RunStore` empty seams (CRUD
    shapes documented; methods land in Phase 3).
  - `skill` module: re-export `starter_skills::SkillSelection`
    behind a default-off `skills` feature (so the workspace
    builds today even though `starter-skills` is a planned
    crate per the agent SCOPE).
  - Crate-root re-exports from `starter-spi`: `Cancel`,
    `Principal`, `SecretString`.
  - `#[non_exhaustive]` on every public enum and config struct.
  - Zero runtime, zero I/O, depends only on `starter-spi`.
- **`starter-flow` crate skeleton** — Cargo.toml + lib.rs with
  module placeholders matching the SCOPE's crate-block layout:
  `graph`, `registry`, `propagator`, `engine`, `run`, `state`.
  Each module is empty except for a doc comment naming the SCOPE
  section that owns it and a Phase-N marker. No engine logic.
- **`starter-flow-nodes` crate skeleton** — Cargo.toml with one
  cargo feature per built-in kind (transform, tool-call,
  ai-agent, branch, merge, gate, subflow, trigger-explicit,
  trigger-event, trigger-schedule, trigger-webhook, http-out,
  log, sleep), one module per feature gated on it, each module
  exposes only the kind id constant in the reserved
  `starter.flow.*` namespace (R10). No `NodeBehavior` impls.
- **`starter-flow-surfaces` crate skeleton** — Cargo.toml +
  lib.rs declaring `FlowAsTool` and `FlowAsService` as empty
  structs with doc comments. No trait impls — those land in
  Phase 3 after the engine exists. Bodies are *absent*, not
  `todo!()`, so a half-built impl cannot escape into a consumer.
- **Workspace wiring** — `starter/Cargo.toml` `members` list
  picks up the four new crates.
- **Dep-tree baseline** — `DOCS/flow/scope/starter-flow-spi-deps.
  baseline.txt` committed; the snapshot the workspace-builds-
  without-adk-rust smoke test compares against.

## Out of scope

- The engine itself (Phase 2 — slot propagator, state machine,
  three-level stop, `Cancel` plumbing, `FlowEvent` stream).
- Persistence — `FlowStore` / `RunStore` impls in
  `starter-store-sqlite` behind the `flow` feature (Phase 3).
- Surface wrappers' bodies — `FlowAsTool::Tool` and
  `FlowAsService::Service` impls (Phase 3).
- The `ai-agent` node kind body (Phase 4 — and D1 stays
  unresolved here, which is on purpose).
- The remaining built-in node-kind bodies — `branch`, `merge`,
  `subflow`, `gate`, `trigger.*`, `http-out`, `log`, `sleep`
  (Phase 5).
- The `starter-ext-flow` adapter — lives in the
  `starter-extensions/` workspace, lands once that workspace
  exists (Phase 6 of the flow SCOPE; its own job).
- Visual canvas / `starter-ui-flow` — Phase 8 in the flow
  SCOPE, marked optional and out of scope until a UI consumer
  asks.
- Any changes to `starter-mcp`, `starter-server`, `starter-cli`
  — those are unmodified per the flow SCOPE Relationship-to-
  existing-crates and What-this-supersedes blocks. R8 and R9
  only matter once Phase 3 surfaces ship.
- Resolving D1 (adk-rust vs lifted Codeless Runner). That is
  Phase 4's entry gate, not this job's.

## Hard rules (load-bearing)

Inherited from the flow SCOPE; restated so the runner re-reads
them every stage even though no engine code lands here yet.

- **R1 — Everything is a Node.** The contracts crate makes this
  enforceable from day one: there is no second "executable
  unit" abstraction in `starter-flow-spi`. Tools (as a node
  kind), agents (as a node kind), triggers (as a node kind) all
  go through `NodeBehavior`.
- **R2 — Slots are the only I/O surface; one write chokepoint.**
  The `WriteSlotOpts` struct lands with `replay: bool` from the
  beginning — the replay-does-not-re-fire-subscribers semantics
  are part of the contract.
- **R3 — The engine is a reader of policies, never an owner.**
  Policy config slots (`session_policy`, `on_failure`,
  `cost_cap`, `safe_state`, `trigger`, `auth`, `timeout`) are
  documented as ordinary slot names in the SCOPE; the contracts
  crate does not enumerate them, the engine reads them from the
  graph at the moment they apply.
- **R4 — Node-kind metadata is static; declared, never
  runtime-templated.** The contracts crate does not parse YAML —
  it only declares the shapes the loader will read.
- **R5 — Node behaviours are stateless.** `NodeBehavior::invoke`
  takes `&self`, never `&mut self`. State lives in slots, in
  the host-provided run/session store, or in the secret store.
- **R6 — Checkpoints are engine-typed.** The contracts crate
  declares `RunStore`'s seam; the engine's own typed
  `RunState` lives in `starter-flow` (Phase 3), not here.
- **R7 — The AI agent is a node kind, not a runtime.** No
  `LlmAgent` / `SequentialAgent` / `LoopAgent` / `GraphAgent`
  appear anywhere in this scaffold; workflow topology is the
  engine's job in Phase 5.
- **R8 — Nodes are not Tools; Tools are one node kind.** The
  contracts crate keeps `starter_spi::Tool` and
  `starter_flow_spi::Node` as two separate traits.
- **R10 — Reverse-DNS ids; namespace ownership enforced.**
  `NodeId` and `KindId` validate on construction. Built-in
  kinds in `starter-flow-nodes` use the reserved
  `starter.flow.*` prefix.
- **R13 — Streaming, cancellation, observability reuse
  existing seams.** `FlowEvent` mirrors `starter_ai::OnEvent`
  shape; `Cancel` re-exported from `starter_spi::ai::Cancel`
  rather than re-invented.

## Constraints

- `starter-flow-spi` has zero non-`starter-spi` deps. Enforced
  by the committed `DOCS/flow/scope/starter-flow-spi-deps.
  baseline.txt` snapshot.
- `starter-flow`, `starter-flow-nodes`, and
  `starter-flow-surfaces` ship `default-features = []`. A
  consumer who does not enable any flow feature pays nothing.
- `starter-flow-nodes` exposes one cargo feature per built-in
  kind. The feature list is locked in stage 1 and the names
  match the SCOPE's "built-in node kinds" enumeration exactly.
- The `skills` re-export in `starter-flow-spi` is behind a
  default-off feature; `starter-skills` is a planned crate (per
  the agent SCOPE) that does not yet exist in the workspace.
- No `adk-rust` dependency anywhere. D1 stays open; the
  scaffold must not preemptively decide either side.
- No engine logic in any crate. `FlowAsTool` / `FlowAsService`
  bodies are absent (not `todo!()`) so the project's
  no-half-finished-implementation rule cannot be cited against
  the scaffold.

## Deliverables

- Four new crates under `starter/crates/`:
  `starter-flow-spi`, `starter-flow`, `starter-flow-nodes`,
  `starter-flow-surfaces`.
- Each crate's `Cargo.toml` + `src/lib.rs` + module skeleton.
- `starter/Cargo.toml` workspace `members` list updated.
- `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` snapshot
  committed.
- `cargo build --workspace --all-features` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo fmt --check` green.
- `cargo tree -p starter-flow-spi --edges normal` matches the
  committed baseline (zero non-starter-spi crates).
- `cargo tree -p starter-flow --edges normal` and
  `cargo tree -p starter-flow-nodes --edges normal` contain no
  `adk-rust` entry — D1 unresolved.

## Open questions (resolve in stage 1)

1. **Default-features posture.** Bias: all three new
   non-contract crates ship with `default-features = []` per
   the SCOPE's "Default-features stay empty per workspace
   policy" line.
2. **Built-in node-kind feature list.** Bias: one cargo feature
   per built-in kind (transform, tool-call, ai-agent, branch,
   merge, gate, subflow, trigger-explicit, trigger-event,
   trigger-schedule, trigger-webhook, http-out, log, sleep) —
   matches the SCOPE's "built-in node kinds" enumeration. Plus
   an aggregate `all-kinds` feature for testing.
3. **Dep-tree baseline location.** Bias:
   `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`,
   updated only when `starter-flow-spi` itself changes (a
   separate, reviewed commit).
4. **`starter-ext-flow` adapter placement.** Bias: out of scope
   for this job because it lives in `starter-extensions/`
   (separate sibling workspace per the flow SCOPE's
   Relationship-to-existing-crates block). Lands once
   `starter-extensions/` ships its workspace; tracked by the
   existing `starter-extensions` job.

D1 (adk-rust vs lifted Codeless Runner for the `ai-agent` node
kind body) is explicitly **not** pre-resolved here — that is
Phase 4's entry gate.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- **`starter-flow-spi` dep purity** —
  `cargo tree -p starter-flow-spi --edges normal` matches the
  committed baseline file. CI gate.
- **No `adk-rust` in the scaffold** —
  `cargo tree -p starter-flow --edges normal` and
  `cargo tree -p starter-flow-nodes --edges normal` contain no
  `adk-rust`. D1 stays open.
- **`SubscribeOpts` and `WriteSlotOpts` placement** — both
  live in `starter-flow-spi`. The engine (Phase 2) consumes
  them; the contracts crate does not depend on the engine.
- **Reserved namespace** — every built-in node-kind id in
  `starter-flow-nodes` starts with `starter.flow.`. R10
  reverse-DNS namespace ownership applies even though no
  external author can contribute yet.
- **No engine code** — `grep -rn 'async fn' crates/starter-flow/src`
  yields no matches; the engine is empty placeholders.
- **No half-finished impls** — `grep -rn 'todo!()' crates/starter-flow-surfaces/src`
  yields no matches; the wrapper structs declare fields TBD
  but expose no trait impls until Phase 3.
