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

Locked in stage 1. Each decision lists the rule it derives from and
the **revisit trigger** — the specific event that should reopen the
question. Anything else is noise; do not reopen on vibes.

### D-S1.1 — Default-features posture

`starter-flow`, `starter-flow-nodes`, and `starter-flow-surfaces`
each ship with `default-features = []`. A consumer that does not
explicitly opt into a flow feature pulls no flow code and pays no
compile cost.

- **Why.** Flow SCOPE Relationship-to-existing-crates block:
  *"Default-features stay empty per workspace policy: a consumer
  who doesn't want flows pays nothing."* Same posture every other
  starter crate already follows (`starter-spi` re-exports, the
  per-integration tool/service crates, `starter-store-sqlite`'s
  `flow` feature).
- **Implication for stage 4 / 5 / 6.** Each crate's `Cargo.toml`
  has `[features] default = []`. No feature is silently on. The
  `starter-flow-nodes` aggregate `all-kinds` feature exists for
  test convenience only — it is **not** in `default`.
- **Revisit when.** A consumer crate inside this workspace
  legitimately needs flow on by default (none today; minimal /
  gh-report / notes examples do not use flows). If that happens,
  the right fix is a feature on the consumer, not on the flow
  crates. Treat any push to flip a flow-crate default as a
  workspace-policy change requiring a separate decision.

### D-S1.2 — Built-in node-kind cargo feature list

`starter-flow-nodes` declares **one cargo feature per built-in
node kind**. The locked list, matching the flow SCOPE
Relationship-to-existing-crates `starter-flow-nodes` block
verbatim:

| Feature             | Kind id                          | Phase |
|---------------------|----------------------------------|-------|
| `transform`         | `starter.flow.transform`         | 2     |
| `tool-call`         | `starter.flow.tool-call`         | 2     |
| `ai-agent`          | `starter.flow.ai-agent`          | 4     |
| `branch`            | `starter.flow.branch`            | 5     |
| `merge`             | `starter.flow.merge`             | 5     |
| `gate`              | `starter.flow.gate`              | 5     |
| `subflow`           | `starter.flow.subflow`           | 5     |
| `trigger-explicit`  | `starter.flow.trigger.explicit`  | 5     |
| `trigger-event`     | `starter.flow.trigger.event`     | 5     |
| `trigger-schedule`  | `starter.flow.trigger.schedule`  | 5     |
| `trigger-webhook`   | `starter.flow.trigger.webhook`   | 5     |
| `http-out`          | `starter.flow.http-out`          | 5     |
| `log`               | `starter.flow.log`               | 5     |
| `sleep`             | `starter.flow.sleep`             | 5     |

Plus one aggregate feature `all-kinds` that enables every kind
above. `all-kinds` is for testing and `cargo check` convenience;
it is **not** in `default` (per D-S1.1).

- **Why.** Flow SCOPE Relationship-to-existing-crates block:
  *"Each behind its own cargo feature."* One feature per kind so
  a consumer pulls only the kinds it uses; this is also what
  keeps the `workspace-builds-without-adk-rust` smoke test honest
  once D1 resolves toward (b) — the `ai-agent` feature is the
  single switch that gates the LLM dep tree.
- **Naming convention.** Lowercase kebab-case. Triggers use
  `trigger-<flavour>` (Cargo feature names cannot contain `.`),
  while the kind id keeps the dotted `starter.flow.trigger.<flavour>`
  form per R10. The mapping is mechanical and documented in the
  table above so a stage-5 author does not have to re-derive it.
- **Revisit when.** A new built-in kind enters the SCOPE's
  Relationship-to-existing-crates `starter-flow-nodes` block. At
  that point add one feature row above; any other change
  (renaming, merging, splitting) requires a SCOPE amendment
  first. Do **not** add features speculatively for kinds the
  SCOPE has not yet adopted.

### D-S1.3 — Dep-tree baseline location and update policy

The `starter-flow-spi` cargo-tree baseline lives at
**`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`** (relative
to the starter repo root).

- **Contents.** The literal output of
  `cargo tree -p starter-flow-spi --edges normal`, captured at
  the commit that adds or modifies `starter-flow-spi`. The file
  itself lands in stage 3 alongside the crate; stage 1 only locks
  the path and the update policy.
- **Update policy.** The baseline updates **only** when
  `starter-flow-spi` itself changes (a new dep added to its
  `Cargo.toml`, a feature flip on `starter-spi`). Updates land in
  the same PR as the change that caused them, never as a drive-by
  refresh. A diff against the baseline that did not come from a
  `starter-flow-spi` edit means a transitive dep regression — fix
  the regression, do not move the baseline.
- **CI gate.** Stage 7's verify list, and every later phase's CI,
  diffs `cargo tree -p starter-flow-spi --edges normal` against
  this file. Same mechanism the `workspace-builds-without-adk-rust`
  test (flow SCOPE Smoke-tests block) uses for `starter-flow` and
  `starter-flow-nodes` — those two get parallel baselines once
  Phase 2 / Phase 5 land code, but they are out of scope for this
  job (the scaffold's stage 7 only diffs `-spi`, since the other
  three are empty placeholders).
- **Revisit when.** A reviewer proposes moving the baseline (e.g.
  into `crates/starter-flow-spi/` itself for locality). Acceptable
  trade-off, but it changes which directory CI watches and which
  doc cross-references break; revisit only if a real
  discoverability problem materialises.

### D-S1.4 — `starter-ext-flow` placement (OUT of scope)

`starter-ext-flow`, the extensions adapter that surfaces
`contributes.nodes` / `contributes.flows`, is **explicitly out of
scope for this job**.

- **Why.** Flow SCOPE Relationship-to-existing-crates block:
  *"Extensions integration lives in a sibling crate inside
  `starter-extensions/` per extensions R13"* — i.e. in a separate
  sibling workspace, not in the starter workspace. Landing it
  here would couple two workspaces and break extensions R13's
  "adapters live with the extensions framework" rule.
- **What lands here instead.** Nothing. The four crates this job
  scaffolds (`starter-flow-spi`, `starter-flow`,
  `starter-flow-nodes`, `starter-flow-surfaces`) are the
  starter-workspace-side of the design; the extensions side lands
  in its own job once `starter-extensions/` ships a workspace
  capable of hosting it.
- **Sequencing.** Flow SCOPE Phasing block puts `starter-ext-flow`
  at Phase 6 (after Phase 5's remaining built-in node kinds).
  That phase opens its own job in the `starter-extensions/`
  workspace; the merge between the two workspaces happens through
  the published crate versions, not by cross-workspace path
  deps.
- **Revisit when.** `starter-extensions/` ships a workspace and
  is ready to host `starter-ext-flow`. At that point the new job
  picks up Phase 6 from the flow SCOPE; nothing in this job's
  outputs needs to change.

### D1 — `ai-agent` body (adk-rust vs lifted Codeless Runner)

**Deferred to Phase 4 entry gate. Not pre-resolved in stage 1.**

- The flow SCOPE Open-questions block (D1) explicitly defers this
  to Phase 4; the scaffold must not preemptively decide either
  side. The `workspace-builds-without-adk-rust` cargo-tree
  snapshot is the CI gate that keeps the option open.
- The `ai-agent` feature in `starter-flow-nodes` (D-S1.2) is the
  switch that will gate whichever choice lands. Stage 5 of this
  job exposes only the kind id constant under that feature; no
  LLM dep enters the tree until Phase 4's job decides between
  `starter-flow-node-loop` and `starter-flow-node-adk`.
- **Revisit when.** Phase 4 opens. Not before.

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
