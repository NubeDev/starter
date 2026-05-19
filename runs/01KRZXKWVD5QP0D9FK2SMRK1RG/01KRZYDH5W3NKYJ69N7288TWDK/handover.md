## Done

- Filled out `crates/starter-flow-nodes` Cargo.toml with one cargo feature per locked built-in kind (transform, tool-call, ai-agent, branch, merge, gate, subflow, trigger-{explicit,event,schedule,webhook}, http-out, log, sleep) plus an `all-kinds` aggregate feature; `default = []`; depends on `starter-flow-spi` with `default-features = false`.
- `lib.rs` declares one `#[cfg(feature = "<kind>")] pub mod <kind>;` per feature (module names use underscores per Rust ident rules; feature names use hyphens to match the stage spec).
- Created 14 module files, each containing only the R10 reverse-DNS `KIND_ID` constant in the `starter.flow.*` namespace and a module-level doc comment naming the SCOPE section(s) that specify the kind's semantics (R1/R7/R8/R3/R12/R13 and the Phase 2/4/5 sections as relevant).
- `cargo check -p starter-flow-nodes --features=all-kinds` green; `cargo check --workspace` green.
- Committed as `e34ccbd` on branch `codeless/starter-flow-scaffold`.

## Next

- Stage 6: per the job goal the remaining flow-skeleton stages cover `starter-flow` engine skeleton and `starter-flow-surfaces` (FlowAsTool / FlowAsService) skeleton. A fresh session will pick this up.

## What you need to know

- Module file naming: Rust idents force underscores (`tool_call.rs`, `trigger_explicit.rs`, etc.); feature names stay hyphenated to match the stage spec and the SCOPE wording.
- `KIND_ID` strings keep the SCOPE wording exactly: `starter.flow.tool-call`, `starter.flow.trigger.explicit`, etc. The `trigger.*` ids use a dot before the variant name (matching the SCOPE prose `trigger.{explicit, event, schedule, webhook}`), not a hyphen.
- Cargo emits a non-fatal warning that `default-features` is ignored for `starter-flow-spi` because the workspace dep entry doesn't declare it. The existing `starter-flow` Cargo.toml has the identical pattern; left as-is for consistency. If a later stage wants the warning silenced, add `default-features = false` to the `[workspace.dependencies] starter-flow-spi = { ... }` entry in the root `Cargo.toml`.
- `lib.rs` retains `#![warn(missing_docs)]`; every `pub const KIND_ID` carries a doc comment so the warning stays clean.

## Open questions

- (none)
