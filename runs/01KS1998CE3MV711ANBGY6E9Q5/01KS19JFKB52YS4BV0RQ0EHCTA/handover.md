## Done

- Created `crates/starter-prefs/` scaffold: Cargo.toml (default-features = [], `routes` feature gate, deps on starter-spi/sqlx/serde/serde_json/utoipa/tracing/iso_currency 0.5)
- `src/lib.rs` declares the four module slots; routes + middleware gated on `routes` feature; each empty submodule documents its owning SCOPE.md section
- Added `crates/starter-prefs` to workspace members between observability and server, and to `[workspace.dependencies]` next to starter-observability
- `cargo check -p starter-prefs` green; committed as `stage 3 — Phase 1 starter-prefs crate scaffold`

## Next

- Stage 4: resolver types + three-layer resolution logic per R3 (likely lives in `resolver.rs`)

## What you need to know

- `iso_currency = "0.5"` is direct, not workspace — first use in the repo (per D-U0.3 it must NOT propagate to starter-spi)
- `routes`/`middleware` modules are `#[cfg(feature = "routes")]` so non-HTTP consumers don't pull axum; routes feature has no transitive deps yet — later stages will add axum/tower under it
- Workspace member list is not strictly alphabetical; slot chosen for grep-cleanliness (observability → prefs → server)

## Open questions

- (none)
