//! Cross-crate smoke tests for the starter-extensions kernel + adapter
//! stack.
//!
//! The crate ships *no* production surface — the lib target exists only
//! because cargo wants something to compile next to `tests/`. Each test
//! file under `tests/` maps one-to-one onto a named scenario in
//! `DOCS/extensions/scope/SCOPE.md` § "Smoke tests (before merging
//! anything)".
//!
//! Scenarios that already have a natural home in another crate's
//! `tests/` directory (e.g. `streaming_response_cancels_promptly` in
//! `starter-ext-server/tests/rest_routes.rs`,
//! `bad_manifest_is_isolated` in
//! `starter-ext-mcp/tests/hello_builtin_end_to_end.rs`, the CLI cancel
//! path in `starter-ext-cli/tests/hello_cli_end_to_end.rs`) stay where
//! they are — the README of this crate cross-references them so the
//! "full sweep" stage-16 check is one `cargo test -p starter-ext-smoke
//! && cargo test --workspace` away.
