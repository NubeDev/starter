## Done

- Added five preferences client methods to `starter-client-rs` (`get_my_preferences`, `patch_my_preferences`, `patch_my_preferences_raw`, `get_org_preferences`, `patch_org_preferences`, `get_units`) with bearer/cookie auth attachment; local `UnitsResponse` DTO mirrors the wire shape.
- Added `prefs {get|set|units}` subcommand to `starter-cli` with `--org`, `--output table|json`, `--field/--value`, and the `auto`→JSON-null inheritance sentinel.
- Refactored dispatch behind `run_prefs_with(out: impl Write)` so tests can capture output.
- Wrote 5 integration tests under `crates/starter-cli/tests/prefs.rs`; spin up the real prefs router on an in-memory sqlite store with an injected Admin Principal. All green; existing `dispatch.rs` tests still green.
- Committed as `stage 7 — …` on `codeless/starter-prefs-i18n`.

## Next

- Stage 8 per WORKFLOW (next session picks it up).

## What you need to know

- `PreferencesPatch` collapses `null`/missing in serde, so the CLI `set` path uses `patch_my_preferences_raw` to preserve explicit-null. The typed `patch_my_preferences` exists per the stage spec but cannot send revert-to-inherit by itself.
- `build_client` uses `try_get_one` for `bearer` because `units` does not declare that arg.
- Test layer uses an `axum::middleware::from_fn` to inject a fixed Admin `Principal` extension; matches the stub style used in starter-prefs's own tests.
- `assert_cmd` and `gag` deliberately not added — kept dev-deps minimal by using an in-process `Vec<u8>` Write sink.

## Open questions

- (none)
