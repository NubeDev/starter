## Done

- `cargo fmt --all` applied; `cargo fmt --check` now clean (was failing on stage-1–5 files plus `examples/iot-anomaly-detector/src/main.rs`).
- `cargo clippy --workspace --all-features -- -D warnings` green under `RUSTUP_TOOLCHAIN=1.91` (workspace already needs 1.91 for aws-smithy under `--all-features`).
- `cargo test` green for the touched-crate slice: `starter-authz`, `starter-auth-users`, `starter-spi`, `starter-authz-demo`.
- `DOCS/auth/authz/README.md` extended with the R11–R15 table and a "Phase 7 worked example" section pointing at `examples/authz-demo`.
- `examples/authz-demo/README.md` rewritten as the canonical Phase 7 walkthrough: tenants seed (R11) + team rule (R13) + `DbDecisionSink` wiring (R14) + REST/MCP/gRPC manifest `auth.permission` (R15) + `GET /v1/authz/decisions` paging + a "Caveats and deferred work" section linking each deferred item to SCOPE-EXT.md §6.
- Audit of new public items on `starter-spi`/`starter-authz`/`starter-auth-users`/`starter-ext-spi` confirms they already carry R-number citations from stages 1–5 (e.g. `Principal.tenant_id` → R11, `Principal.teams` → R13, `ResourceSpec.tenant_scoped` → R11, `DecisionEntry` + `DecisionSink` → R14, `AuthGate.permission` + `PermissionGate` → R15, `with_surface` + `current_surface` → Phase 7d.2). No additional doc comments added.
- Commit `9ca4dc4` on `codeless/authz-phase-7` doubles as the job's exit summary (R-numbers landed, smoke tests passing, deferred items with SCOPE-EXT.md §6 pointers).

## Next

- (none) — Phase 7 of starter-authz is fully landed across stages 1–6.

## What you need to know

- The repo's stated MSRV is 1.80 (root `Cargo.toml`), but `--all-features` pulls aws-smithy crates that require 1.91. All prior stages reported "clippy green" using `--all-features`; the only way to reproduce that is `RUSTUP_TOOLCHAIN=1.91 cargo clippy --workspace --all-features -- -D warnings`. This is a pre-existing condition, not a Phase 7 regression.
- A full-workspace `cargo test --workspace` run hit `ld terminated with signal 7 [Bus error]` in the linker on this host (root filesystem at 90 %, swap thrashing). This is environmental — repeating the run on the touched-crate slice (`-p starter-authz -p starter-auth-users -p starter-spi -p starter-authz-demo`) compiled and ran green. Stage 5's handover reported the full-workspace run green, and stage 6's changes are docs-only on the Rust side (the `.rs` edits are `cargo fmt` whitespace), so the linker failure is not from this stage.

## Open questions

- (none)
