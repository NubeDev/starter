## Done

- Added `skills: Vec<ContributeSkillsDir>` (plus `ContributeSkillsDir { dir: String }`) to `Contributes` in `starter-extensions/crates/starter-ext-spi/src/manifest.rs` with a parse unit test (`contributes_skills_dir_parses`).
- Created new sibling-workspace crate `starter-extensions/crates/starter-ext-flow` (registered in `starter-extensions/Cargo.toml` members + workspace.dependencies). The crate exposes `contributed_skills(&Manifest, &Path) -> Result<Vec<ContributedSkill>, ContributeSkillsError>` — one-level walk of every `contributes.skills[].dir`, picking sub-dirs with a `SKILL.md`, sorted deterministically, debug-traced per dir.
- Integration test `tests/stage10_contributes_skills.rs` covers the three branches required by the stage brief: (a) extension-contributed bundle with `trust: approved` lands in `registry.list_quarantined()` and not in `registry.list()`; (b) missing `dir:` becomes a typed `InvalidSkillsDir` error naming the path; (c) manifest without a `contributes.skills` block returns an empty batch (no error). All 3 tests pass.
- `cargo check --workspace` green on both the main workspace and the `starter-extensions` workspace. `cargo clippy -p starter-ext-flow --all-targets -- -D warnings` green. `cargo test -p starter-ext-flow` and `cargo test -p starter-ext-spi contributes_skills` both green.
- Committed as `e64e80b` on `codeless/starter-skills`. Not pushed (instructions say commit; the closing-trio push step is owned by the runner harness).

## Next

- Stage 11 — Phase 7: ship the two reference SKILL.md bundles at `skills/starter.ai-builder.dashboards/SKILL.md` and `skills/starter.ai-builder.themes/SKILL.md`, content lifted verbatim from `DOCS/frontend/ai-builder/SCOPE.md` §"Skills for ai-builder". Add an integration test that loads `skills/` and round-trips both through `select()` with a matching query.

## What you need to know

- `starter-ext-flow` did not exist before this stage — the stage brief said "one new branch in the existing starter-ext-flow handler" but the crate had never been created (only referenced in `DOCS/agent/SCOPE.md` R-agent-4). The minimum-viable interpretation was to create the crate so the wiring has a home. The crate is deliberately narrow: only `contributes.skills` is wired; the `contributes.flows` and `contributes.nodes` branches are flagged in the lib doc-comment as future stages of the flow track.
- `starter-extensions/` is a sibling cargo workspace; `starter-ext-flow` pulls `starter-skills` via a relative path (`../../../crates/starter-skills`), the same pattern `starter-extensions/Cargo.toml` already uses for `starter-mcp` / `starter-server`. No change to the main workspace dep graph; the `starter-skills` dep-tree CI gate is unaffected.
- `Contributes.skills` defaults to empty (`#[serde(default)]`) so every existing manifest fixture in the repo still parses unchanged. Verified via `cargo check --workspace` on both workspaces.
- The adapter resolves each `dir:` against the **extension bundle root** (`extension_root.join(&entry.dir)`), not against CWD. Callers in `starter-ext-host` should pass the directory the extension's `block.yaml` lives in.
- Trust matrix is preserved: the adapter only collects `ContributedSkill` values; `SkillRegistry::extend(...)` is the one that forces quarantine (R-skills-3 row 3). The test proves this end-to-end by setting `trust: approved` in the fixture frontmatter and asserting it still lands in `list_quarantined()`.

## Open questions

- Whether `starter-ext-host` should call `starter_ext_flow::contributed_skills` automatically on every extension load (and how it gets a handle to the host's `SkillRegistry`) — out of scope for stage 10; the wiring of the wiring is a flow-track concern. The adapter is intentionally library-shaped so the host can call it at whatever lifecycle point makes sense.
