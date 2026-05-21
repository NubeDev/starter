## Done

- Authored `skills/starter.ai-builder.dashboards/SKILL.md` + `prompt.md` + `schema.json` and `skills/starter.ai-builder.themes/SKILL.md` + `prompt.md` + `tokens.json` at the workspace root. Both `SKILL.md` files declare `trust: approved`, list their resources as `file://...` URIs, and use valid reverse-DNS `id` / `allowed_tools`.
- Added `crates/starter-skills/tests/stage11_reference_bundles.rs` — 3 tokio tests against the real `SkillRegistry` + `InMemoryApprovalStore` + `KeywordSkillSelector`, loading the workspace `skills/` dir via `load_dir`. All three pass; the full `cargo test -p starter-skills` suite (10 tests across 3 files + parser unit tests) is green.
- Committed as `a2e4ac8` on `codeless/starter-skills` (commit message starts with the stage title).

## Next

- Stage 12 of 12 of the starter-skills job remains (per the "Stage 11 of 12" header). A fresh session will pick it up.
- ai-builder job (separate) will eventually wire these bundles into the Phase 5 smokes — explicitly out of scope for this stage.

## What you need to know

- `DOCS/frontend/ai-builder/SCOPE.md` does **not** exist in this repo. The stage brief asks for the bundle content to be "lifted verbatim" from that file's §"Skills for ai-builder" section; since the file is missing, the bundle content was authored here to fit the parser surface (reverse-DNS ids, valid `KindId` allowed tools, `file://` resources) and the `KeywordSkillSelector` first-overlap rule. The commit message flags this and notes that the content should be replaced verbatim once the ai-builder SCOPE lands. If a future stage requires byte-exact wording, this will need re-syncing (which will also bump the `bundle_hash` and any approval-store rows keyed on the old hash).
- `KeywordSkillSelector` iterates candidates in `BTreeMap<SkillId, _>` order. `starter.ai-builder.dashboards` sorts before `starter.ai-builder.themes`, so any query whose tokens overlap *both* descriptions (or *neither*, via the fallback) routes to dashboards. The themes routing test deliberately uses `"restyle palette"` — both tokens appear in the themes description and neither in the dashboards description.
- Resource hashes for the bundled `prompt.md` / `schema.json` / `tokens.json` are now baked into each skill's `bundle_hash` via `hash_bundle` (R-skills-2). Editing those files is a load-bearing change; the Phase 4b on-mount check will re-quarantine on drift.
- `KindId::new` requires `[a-z0-9_-]` per segment with at least two `.`-separated segments — used `starter.mcp.call` and `starter.flow.transform` for `allowed_tools`. These ids do not need to resolve against a real `ToolRegistry` for the parser to accept them; intersection with the host registry happens at run time (agent R3).

## Open questions

- Should a follow-on prose stage track the "lift verbatim from ai-builder SCOPE" obligation, or does it block on the ai-builder job creating `DOCS/frontend/ai-builder/SCOPE.md` first? The commit flags the substitution but doesn't open an issue; if the project uses GitHub issues, one may want to be filed.
