## Done

- Added `RubixNodeYaml::allowed_tools()` accessor + `ALLOWED_TOOLS_KEY` const in `rubix-flows/src/yaml.rs`; new `LoadError::AllowedTools` variant in `error.rs`.
- `convert()` now explicitly writes the validated full list into `NodeDecl.settings.allowed_tools` (not just `[0]`).
- New integration test `allowed_tools_multi_entry_list_round_trips_through_convert` in `tests/load_test.rs` covers a 4-tool list through both surface and post-convert layers.
- `cargo test -p rubix-flows` green (4 tests); `cargo check -p rubix-agent` clean.
- Committed as `phase A.3 — bundled-flow allowed_tools[] population` (065e36a).

## Next

- Stage 4 of 16 picks up next session per the job plan (not started).

## What you need to know

- AiAgentNode (`crates/starter-flow-nodes/src/ai_agent.rs`) already reads `allowed_tools` from settings as a `Vec<String>` via the `ALLOWED_TOOLS_SLOT` config slot, so the typed list now flows there automatically.
- `rubix-agent/src/boot/mcp/register.rs::primary_tools_from_triples` still selects `allowed_tools[0]` as the deterministic "primary tool"; that behaviour is unchanged and intentional for now.
- `serde_yaml` is the existing dep; no new crates added.

## Open questions

- (none)
