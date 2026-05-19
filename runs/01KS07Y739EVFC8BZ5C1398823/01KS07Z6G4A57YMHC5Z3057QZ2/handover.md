## Done

- Appended D1d–D1g to DOCS/flow/scope/SCOPE.md "Decisions made":
- D1d: Phase 2 catch-up smokes live under `crates/starter-flow/tests/`, not `crates/smoke-tests/` (owned by tools SCOPE Stage 9).
- D1e: `transform` body uses registered Rust closures indexed by `config.fn_id`; no rhai/starlark/wasm dep on `starter-flow-nodes`.
- D1f: `tool-call` body resolves its `Tool` via an `Arc<dyn ToolRegistry>` threaded through the run — no global static.
- D1g: R3 grep-contract test at `crates/starter-flow/tests/r3_no_policy_match_arms.rs` scans crate's own `src/` for literal `match` arms on the seven policy slot names (`session_policy`, `on_failure`, `cost_cap`, `safe_state`, `trigger`, `auth`, `timeout`); doc-comment / string-literal hits fine, match-arm hits stage-fail.
- Each decision includes an explicit revisit trigger.
- Committed as `c13d84d` on branch `codeless/starter-flow-engine-finish` with message starting "stage 1 — lock the catch-up boundary before code lands".

## Next

- Stage 2 picks up in a fresh session. Likely first code stage: implement the `transform` NodeBehavior body in `crates/starter-flow-nodes/src/transform.rs` per D1e (registered Rust closure indexed by `config.fn_id`).

## What you need to know

- Existing relevant files already in tree: `crates/starter-flow-nodes/src/transform.rs`, `crates/starter-flow-nodes/src/tool_call.rs` (stubs from the merged starter-flow-engine PR #5).
- `crates/starter-flow/` has no `tests/` directory yet — Stage 2+ creates it for both the smokes and the R3 grep test.
- The substrate decisions in D1e/D1f deliberately mirror the merged starter-flow-engine job Stage 1 sub-decisions so the catch-up bodies don't drift.
- The R3 grep test (D1g) is intentionally dep-free: line-oriented scan over `crates/starter-flow/src/**/*.rs` is sufficient; no syn / cargo-expand.

## Open questions

- (none)
