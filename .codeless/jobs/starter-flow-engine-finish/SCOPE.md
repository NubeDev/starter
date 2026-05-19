# Scope — starter-flow-engine-finish

> Source of truth: [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> §"Phase 2 — `starter-flow` engine (in-memory stores)" and the
> §"Smoke tests" block. This file is the per-job brief; intentionally
> short. When this file disagrees with the source-of-truth SCOPE,
> that doc wins.

## Goal

Close the Phase 2 gap. The merged
[`starter-flow-engine`](../starter-flow-engine/) sibling job (PR #5)
shipped stages 1 through 7 of a 12-stage plan — the engine
substrate (graph + propagator + registries + engine state machine +
run lifecycle) landed end-to-end. The remaining Phase 2 deliverables
that prove the engine actually works did not:

- `transform` `NodeBehavior` body (still a 13-line `KIND_ID` stub).
- `tool_call` `NodeBehavior` body (same — 13-line stub).
- "One write chokepoint" SCOPE smoke test.
- "Engine is reader of policies" SCOPE smoke test.
- R3 grep-contract test (no hardcoded policy match arms in the
  engine crate's source).

This is a **catch-up job, scoped to exactly those four pieces** and
the workspace verify that confirms the dep-tree gates still hold.
No engine refactors, no new node kinds beyond the two Phase 2 names,
no dep changes beyond what the bodies legitimately need.

After this job lands, Phase 2 of the flow SCOPE is honestly green:
the two Smoke-tests block items pass under `cargo test`, the
adk-rust-free invariant has an automated CI gate, and Phase 3
(persistence + surface wrappers) has a real flow to test through
`starter-mcp` instead of a synthetic stub.

## In scope

- **`transform` body** in `crates/starter-flow-nodes/src/transform.rs`
  behind the existing `transform` feature flag. Pure-fn semantics
  per the flow SCOPE Phase 2 block. Uses the Rust-closure
  substrate decided in the prior
  [`starter-flow-engine` job](../starter-flow-engine/SCOPE.md)
  stage-1 sub-decision — a registered closure indexed by a
  `config.fn_id` slot value. No `rhai` dep on `starter-flow-nodes`.
- **`tool_call` body** in `crates/starter-flow-nodes/src/tool_call.rs`
  behind the existing `tool-call` feature flag. Looks up the
  `Tool` via an `Arc<dyn ToolRegistry>` threaded through the run
  (same substrate as the prior sibling job). Auth is applied by
  the adapter, not by the node body, per extensions R13.
- **"One write chokepoint" smoke** as
  `crates/starter-flow/tests/smoke_one_write_chokepoint.rs`.
  Three writers (REST-style stub task, CLI-style sync call,
  propagator tick) all enter `GraphStore::write_slot`; tracing
  span count == 3; `SlotChanged` envelopes == 3 with three
  distinct values. If any writer bypasses the chokepoint, the
  test fails.
- **"Engine is reader of policies" smoke** as
  `crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs`.
  Two writable output nodes with `safe_state` policies (`fail-safe(0)`
  and `hold-last`); `Engine::stop()` is called; both outputs
  receive their safe-state writes; a grep-assert inside the same
  test rejects hardcoded policy match arms outside doc comments.
- **R3 grep-contract test** as
  `crates/starter-flow/tests/r3_no_policy_match_arms.rs`. Reads
  every `.rs` under `crates/starter-flow/src/`, strips comments
  and string literals, refuses any literal identifier match-arm
  on `session_policy | on_failure | cost_cap | safe_state |
  trigger | auth | timeout`. The single legitimate hit
  (`fn safe_state` on the `WritableOutput` trait — an R12 hook,
  not a policy match) is allow-listed inline with a justification
  comment. Span names with these words are fine; match arms are
  not.
- **Workspace verify + dep-tree gates re-confirmed.** `cargo
  build --workspace --all-features`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`. `cargo
  tree -p starter-flow --edges normal | grep adk-rust` returns
  empty; same for `starter-flow-nodes`. `cargo tree -p
  starter-flow-spi --edges normal` matches
  `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  byte-for-byte. None of the four flow crates path-dep onto
  `starter-mcp` / `starter-server` / `starter-cli` (Phase 3 wires
  the other way).

## Out of scope

- **Phase 3 work** — `FlowStore` / `RunStore` / `SessionStore`
  impls in `starter-store-sqlite`; `FlowAsTool` / `FlowAsService`
  bodies. That is the next job, drafted separately once this one
  lands.
- **Engine refactors.** The merged Phase 2 work is the substrate
  this job sits on; touching `engine.rs` / `run.rs` /
  `propagator.rs` / `graph.rs` / `registry.rs` for anything
  other than narrow tests is out. If a real defect is found,
  surface it as an issue + a separate PR.
- **The remaining built-in node kinds.** `branch`, `merge`,
  `gate`, `subflow`, `trigger.{explicit, event, schedule,
  webhook}`, `http-out`, `log`, `sleep`. Phase 5 of the flow
  SCOPE; not here.
- **`ai-agent` body.** Phase 4 + D1 resolution. The kind module
  stays a `KIND_ID` constant.
- **Modifications to `starter-flow-spi`.** Phase 1 froze the
  contracts; this catch-up does not touch them. Stage 7's
  baseline diff catches accidental drift.
- **Adding the `starter-smoke-tests` crate.** That crate exists
  in the workspace for the tools SCOPE Stage 9 smokes (slack /
  telegram / gmail providers); the Phase 2 flow smokes live as
  integration tests on `starter-flow` itself to avoid scope
  collision and to keep the engine-crate's own tests local.
- **Modifications to the closing-trio code in `codeless`.**
  This job only touches the starter repo per the prior sibling
  job's branch convention.

## Hard rules (load-bearing — inherited from flow SCOPE)

Restated so the runner re-reads them every stage:

- **R2 — One write chokepoint.** Both new node bodies write
  through `GraphStore::write_slot`. The smoke in stage 5 is the
  enforceable contract.
- **R3 — Engine reads policies, never owns them.** The grep-test
  in stage 6 is the enforceable contract; the smoke in stage 5
  also asserts on the SCOPE's "no hardcoded policy registry"
  property.
- **R5 — Node behaviours are stateless.** `transform::invoke`
  and `tool_call::invoke` take `&self`, never `&mut self`. State
  lives in slots, in the host-provided registries, or in the
  run/session store.
- **R7 — AI agent is a node kind, not a runtime.** This job does
  NOT add the `ai-agent` body. The outer-run skill-selection seam
  is already wired by the prior sibling job; this catch-up does
  not touch it.
- **R8 — Nodes are not Tools; Tools are one node kind.** The
  `tool_call` body wraps `starter_spi::Tool` lookups from the
  host-provided `ToolRegistry`. The two traits stay separate.
- **R10 — Reverse-DNS ids enforced.** The existing `KIND_ID`
  constants in both files stay byte-for-byte. The `tool_call`
  body validates the `tool_id` it reads from a config slot as
  a `KindId` (same R10 reverse-DNS namespace ownership).
- **R12 — Three-level stop + safe-state.** Stage 5's "Engine is
  reader of policies" smoke uses the `WritableOutput` trait the
  engine substrate already exposes; the smoke is what verifies
  the safe-state walk actually runs on `Engine::stop()`.
- **R13 — Streaming + cancellation + observability.** The
  `tool_call` body's Cancel-propagation test in stage 4 is the
  load-bearing case — a `Cancel` firing mid-tool-call aborts
  the in-flight call within bounded time.

## Constraints

- **Existing engine deps stay.** No new top-level deps on
  `starter-flow`. `starter-flow-nodes` may add the minimal deps
  the two bodies need (likely none — `serde_json` is already
  workspace-available; the closure-registry shape needs no new
  dep). Stage 7's dep-tree check enforces this.
- **No `adk-rust` anywhere.** Stage 7 grep on the cargo trees.
- **`default-features = []` posture stays.** The `transform`
  and `tool-call` feature flags ship as default-off per the
  scaffold job's D-S1.1.
- **MSRV 1.78** (workspace). `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`
  non-negotiable.
- **Tests live with the code.** Stages 3 and 4 commit bodies and
  unit tests together; stage 5 commits the two SCOPE smokes;
  stage 6 commits the R3 grep test.
- **One logical batch per stage.** Stage 3 = transform; stage 4
  = tool_call; stage 5 = two SCOPE smokes; stage 6 = R3 grep
  test. No bundling.
- **Smoke tests live in
  `crates/starter-flow/tests/`,** not in the
  `starter-smoke-tests` crate (which the tools SCOPE owns).
- **`starter-flow-spi` baseline unchanged.** Stage 7 diff.

## Decisions

Locked in stage 1. Each decision lists the rule it derives from
and the **revisit trigger** — the event that should reopen the
question. Anything else is noise.

### D-F2F.1 — Smoke-test location: `crates/starter-flow/tests/`

The two Phase 2 SCOPE smoke tests and the R3 grep test live as
integration tests on the `starter-flow` crate, under
`crates/starter-flow/tests/`. They do NOT live in the existing
`crates/smoke-tests/` (`starter-smoke-tests`) crate.

- **Why.** `crates/smoke-tests/` is owned by the tools SCOPE
  Stage 9 work — it tests the slack / telegram / gmail provider
  family. Adding flow smokes there couples two unrelated SCOPEs
  through a single test crate, and the `[dev-dependencies]`
  list in its `Cargo.toml` would pull a Cargo.toml from one
  SCOPE into the dep tree of another. Integration tests on the
  engine crate are local, run automatically under `cargo test
  -p starter-flow`, and require no workspace-member additions.
- **Implication for stage 5.** Both smoke files are siblings of
  `crates/starter-flow/src/`, mirror the SCOPE Smoke-tests
  block one-for-one in file names, and are discoverable as
  `cargo test -p starter-flow --test smoke_one_write_chokepoint`
  and `--test smoke_engine_is_reader_of_policies`.
- **Revisit when.** Phase 3 (persistence + surfaces) lands and
  the smoke catalog grows past three or four flow-specific
  tests. At that point a dedicated `crates/starter-flow-smokes/`
  workspace member is reasonable; until then in-tree integration
  tests are simpler.

### D-F2F.2 — `transform` substrate: Rust closure registry

The `transform` body uses a Rust-closure substrate, NOT `rhai` or
any embedded scripting. A new public trait
`TransformFunctionRegistry` lives in `starter-flow-nodes`; a
default `StaticTransformFunctionRegistry` impl backs it for the
common case. Each `Transform` node carries a `config.fn_id`
slot whose value names the registered closure to apply.

- **Why.** Matches the prior sibling job's stage-1 sub-decision
  verbatim. `rhai` adds a meaningful dep tree and the Phase 2
  smoke ("transform sums two input slots into one output") does
  not need scripting. The Phase 5 richer `transform` body
  (`rhai`, `starlark`, embedded `wasm`) is a separate decision
  in a separate job.
- **Implication for stage 3.** The crate exports the trait and
  the static impl as public surface so consumers (and the smoke
  tests in stage 5) can register their own closures. The trait
  shape is `register(fn_id, Arc<dyn Fn(SlotValue) ->
  Result<SlotValue, NodeBehaviorError> + Send + Sync>)` /
  `lookup(fn_id) -> Option<…>`.
- **Revisit when.** A Phase-5 job opens for the richer
  `transform` language. At that point either (a) the registry
  grows a `RhaiTransform` impl alongside the static one, or (b)
  the registry trait shifts to accommodate compiled expressions.
  No change forced by this catch-up.

### D-F2F.3 — `tool_call` `ToolRegistry` injection: per-run via engine construction

The `tool_call` body looks up the `Tool` via an `Arc<dyn
ToolRegistry>` threaded through the run by the engine's
construction args, NOT via a global static. Matches the
substrate from the prior sibling job stage 1.

- **Why.** Single source of truth per R8 ("Tool is the
  MCP-callable primitive; Node is engine-internal"). The same
  registry `starter-mcp` already serves becomes the registry
  the `tool-call` node consults — when Phase 3 mounts
  `FlowAsTool`, no second registry needs reconciling.
- **Implication for stage 4.** The engine's existing
  construction surface in `starter-flow` exposes
  `with_tool_registry(Arc<dyn ToolRegistry>)`; the `tool_call`
  body reads it from the per-run context the engine threads
  through. If the prior sibling job did not actually wire this
  through to the per-invoke context, stage 4 lands the
  threading at the same time (small change; in scope).
- **Revisit when.** A use case surfaces where two distinct
  ToolRegistries need to coexist in the same engine (e.g. a
  sandboxed extension's tools vs the host's). At that point
  the per-flow registry overlay lands as a separate decision;
  per-run pass-through stays the default.

### D-F2F.4 — R3 grep-test allow-list

The R3 grep test (`r3_no_policy_match_arms`) allow-lists exactly
one occurrence: the `safe_state` identifier as the
`WritableOutput` trait method name in `engine.rs`. That hit is
the legitimate R12 hook — `WritableOutput::safe_state(&self) ->
SlotValue` returns the value to drive on stop; it is not a
match arm on a policy slot. Every other hit on the seven
identifier names is a stage-fail.

- **Why.** R12 names `safe_state` as the hook the engine reads
  during the stop walk; the trait method's name has to be
  `safe_state` to match the SCOPE. The grep-test would
  otherwise have a false positive on every workspace build.
- **Implication for stage 6.** The allow-list is a single
  hard-coded line in the test source carrying a justification
  comment quoting R12. If a future engine change adds a second
  legitimate hit, the allow-list grows by a single line with
  its own justification — not a wildcard exemption.
- **Revisit when.** The `WritableOutput` trait is renamed or
  the `safe_state` method moves to a different file. Update
  the allow-list in the same PR as the rename.

## Cross-cutting checks the runner must keep honest

- **R2 chokepoint integrity** — `grep -rn 'fn write_'
  crates/starter-flow/src` shows exactly one `write_slot`
  function on `GraphStore`. Stage 5's "One write chokepoint"
  smoke is the runtime contract.
- **R3 policy-discipline** — stage 6's grep-test is the
  enforceable contract. Pre-existing source already complies;
  the test ensures it stays that way.
- **R5 `&self` discipline** — both new bodies'
  `NodeBehavior::invoke` impls take `&self`, never `&mut
  self`. Clippy's `needless_borrows_for_generic_args` plus
  manual inspection in stage 3 + stage 4.
- **No `adk-rust`** — `cargo tree -p starter-flow --edges
  normal | grep adk-rust` returns empty. Same for
  `starter-flow-nodes`. Stage 7 CI gate.
- **`-spi` baseline unchanged** — `cargo tree -p
  starter-flow-spi --edges normal | diff -
  DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` is empty
  at stage 7 end.
- **Cancel propagation** — stage 4's tool_call test asserts
  Cancel fires within 200 ms.
- **Smoke-test idempotency** — both stage-5 smoke files use
  the same `WriteSlotOpts` posture (no `force=true` unless the
  semantics demand it) so the tests don't accidentally cover
  for a broken short-circuit.

## Deliverables

- `crates/starter-flow-nodes/src/transform.rs` populated with
  `NodeBehavior` impl + `TransformFunctionRegistry` trait +
  `StaticTransformFunctionRegistry` impl + unit tests.
- `crates/starter-flow-nodes/src/tool_call.rs` populated with
  `NodeBehavior` impl + unit tests using a `MockTool`.
- `crates/starter-flow/tests/smoke_one_write_chokepoint.rs`.
- `crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs`.
- `crates/starter-flow/tests/r3_no_policy_match_arms.rs`.
- `cargo build --workspace --all-features` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo fmt --check` green.
- `cargo tree -p starter-flow --edges normal` and `cargo tree
  -p starter-flow-nodes --edges normal` contain zero
  `adk-rust`.
- `cargo tree -p starter-flow-spi --edges normal` matches
  `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  byte-for-byte.

## Open questions (resolve in stage 1)

1. **`transform` panic surfacing.** Bias: a panic inside the
   registered closure is caught by a `std::panic::catch_unwind`
   wrapper at `invoke` time and surfaced as
   `FlowEvent::NodeFailed { reason: "transform panicked" }`.
   The wrapper costs near-zero on the happy path.
2. **`tool_call` config slot naming.** Bias:
   `config.tool_id` carries the `KindId` string; the test suite
   in stage 4 documents this in the `MockTool` setup. If the
   prior sibling job already named a different slot, match
   that.
3. **Span attribute granularity.** Bias: include
   `principal_id_hash` (not the raw `Principal`) on the
   `tool_call.invoke` span so traces don't leak identity. Match
   the workspace's existing `starter-spi`/`starter-server`
   pattern on principal logging.
4. **R3 grep-test traversal.** Bias: pure-Rust line-by-line
   tokeniser inside the test (strip `//`/`///`/`//!` comments
   and string literals via a small state machine). No external
   dep (`syn` is overkill; `regex` is overkill). The test runs
   under one second on the current `starter-flow/src/` size.
