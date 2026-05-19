## Done

- Implemented `crates/starter-flow/src/registry.rs` with `NodeKindRegistry` (KindId → `Arc<dyn NodeBehavior>`, `tokio::sync::RwLock<HashMap<…>>`-guarded) exposing `register` (refuses `starter.flow.*` reserved prefix per R10), `register_builtin` (host-only path that accepts the reserved prefix), `deregister`, `lookup`, and `len`/`is_empty` helpers. Duplicate KindId registration is refused on both paths.
- Implemented `FlowRegistry` (FlowId → `Vec<FlowDefinition>`, RwLock-guarded) with `put`, `lookup(flow, revision)`, `head(flow)`, `revisions(flow)`. Multiple revisions per flow supported; duplicate `(flow, revision)` pair returns `DuplicateRevision`.
- `FlowDefinition` is a Phase-2 `#[non_exhaustive]` placeholder carrying `FlowId` + `FlowRevisionId`; Phase 3 adds nodes/links/auth without a breaking change.
- `RegistryError` is `#[non_exhaustive]` + `thiserror`-derived + `PartialEq` for tests. Added `thiserror = { workspace = true }` to `crates/starter-flow/Cargo.toml`.
- All 5 stage-required unit tests pass (reserved-prefix refusal, duplicate KindId, lookup-after-register/deregister, multiple revisions per flow). `cargo test -p starter-flow` → 15 passed; `cargo clippy -p starter-flow --all-targets -- -D warnings` clean; `cargo fmt` applied.
- Committed as `6db50c0` on branch `codeless/starter-flow-engine` with message starting "stage 5 — NodeKindRegistry and FlowRegistry in crates/starter-flow/src/registry.rs".

## Next

- Stage 6 of 12 per the SCOPE Phase 2 plan (engine state machine `Starting → Running → Pausing → Paused → Resuming → Stopping → Stopped` per R12, in `crates/starter-flow/src/engine.rs`) — a fresh session picks it up.

## What you need to know

- Reserved-prefix policy enforced here is `starter.flow.*` only; the wider R10 reservation surface (`sys.*`, `starter.*`, `flow.*`) is deferred to the `starter-ext-flow` adapter boundary in Phase 6, as noted in the `RESERVED_KIND_PREFIX` doc comment.
- `register_builtin` is a regular `pub` method — host-only is enforced by convention/review (same posture starter-spi uses for its host-only seams). If a future stage wants compile-time enforcement, the seam to harden is here.
- `FlowDefinition` is intentionally minimal. The engine's runnable `FlowTopology` (in `propagator.rs`) is currently constructed directly from test fixtures; wiring `FlowRegistry → FlowTopology` is a Phase 3 task once `FlowDefinition` carries nodes/links.
- `RegistryError: PartialEq` was added so tests can `matches!` on the variants ergonomically; if Phase 3 adds a variant whose payload isn't `Eq`, that derive will need to drop.

## Open questions

- (none)
