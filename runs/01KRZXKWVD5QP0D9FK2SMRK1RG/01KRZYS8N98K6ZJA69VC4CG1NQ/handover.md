## Done

- Verified the four Phase 1 crates (starter-flow-spi, starter-flow, starter-flow-nodes, starter-flow-surfaces) are listed in starter/Cargo.toml [workspace.members]; stages 2–5 had already added them.
- Fixed `cargo build --workspace --all-features`: gated the starter-flow-spi `SkillSelection` re-export behind `cfg(all(feature = "skills", any()))` so `--all-features` no longer trips the unresolved `starter_skills` crate. Updated both `crates/starter-flow-spi/src/skill.rs` and `crates/starter-flow-spi/Cargo.toml` comments to record the placeholder-until-starter-skills-ships intent.
- Fixed `cargo clippy --workspace --all-targets -- -D warnings`: added `#[allow(clippy::module_inception)]` on `crates/starter-spi/src/service/mod.rs` for the pre-existing `service::service` inception, manually fixed a doc lazy-continuation lint in `crates/starter-service-slack/tests/socket_mode.rs`, and ran `cargo clippy --fix` for the rest of the workspace (~30 files modified across starter-auth-oauth, starter-mcp, starter-tauri, examples/notes, etc.).
- `cargo fmt` applied; `cargo fmt --check` is clean.
- Confirmed `cargo tree -p starter-flow-spi --edges normal` matches `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` byte-for-byte (`diff` exit 0).
- Confirmed `cargo tree -p starter-flow --edges normal` and `cargo tree -p starter-flow-nodes --edges normal` contain no `adk-rust` entry (D1 still unresolved per stage 0).
- Confirmed none of the four flow crates pull in starter-mcp / starter-server / starter-cli via grep on their `cargo tree` outputs.
- Committed everything as 597bc49 with a message starting `stage 7:`.

## Next

- Phase 1 of DOCS/flow/scope/SCOPE.md is now scaffolded; the next phase (engine internals — graph, registry, propagator, engine state machine) is Phase 2 per the SCOPE.
- The starter-ext-flow adapter still lives in the separate `starter-extensions/` workspace and is intentionally NOT touched here.

## What you need to know

- Branch is `codeless/starter-flow-scaffold`, tip 597bc49. No remote push performed.
- The `skills` feature on starter-flow-spi is now a declared-but-inert placeholder; when `starter-skills` lands as a workspace member, drop the `any()` half of the cfg gate in `skill.rs` and add the matching optional dependency in `Cargo.toml`.
- The clippy `--fix` rollup touched files outside the new flow crates because clippy is workspace-wide; those changes were pre-existing lint debt the stage-7 smoke gate forced into scope. Review them in the diff if a future stage is sensitive to them.
- Pre-existing clippy lint `clippy::module_inception` on `starter-spi/src/service/service.rs` is silenced with an `#[allow]` rather than renamed, since renaming is out of scope for this stage.

## Open questions

- (none)
