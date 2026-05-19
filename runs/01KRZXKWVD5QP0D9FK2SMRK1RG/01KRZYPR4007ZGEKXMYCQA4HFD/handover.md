## Done

- Added `pub struct FlowAsTool { /* TODO Phase 3 — fields per R8 */ }` and `pub struct FlowAsService { /* TODO Phase 3 — fields per R9 */ }` to `crates/starter-flow-surfaces/src/lib.rs` with doc comments naming SCOPE §R8 "Nodes are not Tools — Tools are one node kind" and §R9 "Flows are first-class Tools and first-class Services".
- Set `default-features = false` on both `starter-flow-spi` and `starter-spi` deps in `crates/starter-flow-surfaces/Cargo.toml`.
- Verified `cargo check -p starter-flow-surfaces` green.
- Committed as `38b8a29` with message starting `starter-flow-surfaces crate skeleton — …`.

## Next

- Stage 7 of 7 — the final stage of the Phase 1 scaffold per the job goal (workspace-wide check, README/TODO/CHANGELOG bookkeeping, or whatever the stage spec lays out). A fresh session picks it up.

## What you need to know

- No trait impls were added — keeps the structs from accidentally satisfying `Tool` / `Service` bounds before Phase 3.
- Bodies are absent rather than `todo!()` per CLAUDE.md's no-half-finished-implementation rule, as the stage spec requires.
- Cargo emits four pre-existing warnings of the form "`default-features` is ignored for starter-flow-spi/starter-spi, since `default-features` was not specified for `workspace.dependencies.…`". Same pattern already fires from `starter-flow` and `starter-flow-nodes`, so it's a workspace-level concern, not stage 6 scope. Fixing it means adding `default-features = true` (or `false`) to the workspace dep entries in the root `Cargo.toml`.
- `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]` already at the crate root; new items carry doc comments so missing_docs stays silent.

## Open questions

- (none)
