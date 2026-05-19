## Done

- Added `crates/starter-flow-spi/` with full Phase 1 contracts: `NodeBehavior` (async `invoke` + `on_lifecycle`, `&self` per R5), reverse-DNS-validated `NodeId` / `KindId` / `FlowId` newtypes (R10), `SlotRef`, `SlotValue` (`#[non_exhaustive]`, Null/Bool/Int/Float/String/Bytes/Json), `SlotMap` alias, `GraphStore` trait with `write_slot`/`read_slot`/`subscribe`, `WriteSlotOpts { replay: bool }` (R2), `SubscribeOpts`, `SubscriptionStream`, `FlowEvent` (`#[non_exhaustive]`, all 7 variants), `FlowStore`/`RunStore` as empty Phase-3 seams, `FlowRevisionId`/`RunId` UUID newtypes, `Cancel`/`Principal`/`SecretString` re-exports from `starter-spi`, `SkillSelection` re-export gated behind default-off `skills` feature.
- `#[non_exhaustive]` applied to every public enum and config struct.
- Empty crate skeletons added: `starter-flow`, `starter-flow-nodes`, `starter-flow-surfaces`.
- All four crates added to workspace `members` and `workspace.dependencies`.
- `cargo build --workspace` succeeds; `cargo test -p starter-flow-spi` passes (reverse-DNS validation tests).
- `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` committed via `cargo tree -p starter-flow-spi --edges normal`.
- Committed as "Phase 1 — starter-flow-spi contracts crate".

## Next

- Stage 4 should begin Phase 2: in-memory `GraphStore` + slot propagator + `NodeKindRegistry` / `FlowRegistry` + engine state machine in `starter-flow`, plus first two built-in kinds (`transform`, `tool-call`) in `starter-flow-nodes`.

## What you need to know

- `starter-flow-spi` Cargo.toml does NOT declare a `starter-skills` dep — the `skills` feature is a stub. When `starter-skills` becomes a workspace member, add an optional `[dependencies.starter-skills]` entry and wire `skills = ["dep:starter-skills"]`. Enabling `skills` today fails at link time on purpose (gate against half-wired releases).
- `NodeError::Other` wraps a private `anyhow_compat::Error` boxed-error so this crate doesn't pull `anyhow`. Replace if/when the engine fixes a richer error shape.
- `GraphStore::subscribe` returns `BoxStream<GraphEventEnvelope>`; the concrete `GraphEvent` payload is deliberately deferred to the engine crate to avoid committing it from contracts.
- `validate_reverse_dns` is `pub(crate)` and shared by `NodeId`/`KindId`/`FlowId`. Rules: ≥2 dot-separated segments, each starting with `[a-z]`, body `[a-z0-9_-]`.
- `FlowEvent` stringifies underlying `NodeError`/`FlowError` (the `error: String` field) so the event stream stays `Serialize` without committing the error variant shape.
- `starter-ext-flow` is intentionally absent — it belongs in the `starter-extensions/` workspace per SCOPE.

## Open questions

- D1 (adk-rust vs lifted Codeless `Runner` for the `ai-agent` body) remains deferred to Phase 4 entry gate per stage 1's decisions.
