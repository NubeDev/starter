## Done

- Added `rubix/crates/rubix-agent/src/routes/{mod.rs,tools.rs}` exposing `POST /api/v1/tools/{tool_id}` with a 17-line handler that extracts `Json<Value>`, dispatches via the boot-time tool registry by `definition().name`, and shapes `Result<Value, Error>` into a JSON response with status code per `Error` variant (Invalid→400, NotFound→404, Unauthenticated→401, Forbidden→403, Conflict→409, Internal→500, non-exhaustive `_`→500).
- Wired `starter-i18n` `AcceptLanguageLayer` so the handler reads `LocaleCtx`; `?render=server` calls `MessageBundle::render_diagnostic` against `prefs_from_locale(&lang)` (re-used from `boot::mcp`) and adds a `rendered_summary` field beside the raw `summary` Diagnostic. Default (no query) leaves the response with raw Diagnostic JSON only.
- Refactored `src/health.rs` to expose `healthz_router()` + a `serve(bind, Router)` entry point; `src/main.rs` now composes `healthz_router().merge(routes::tools::router(state))` and the original startup log + `_mcp_router` keep-alive are preserved.
- Added `src/bin/rubix_admin/system/{mod.rs,disk.rs}` registering `rubix-admin system disk [--mount …] [--json]`. The verb calls `rubix_tools::system::disk::probe()` in-process, renders the `Diagnostic` through `MessageBundle::render_diagnostic` against `$LANG` (POSIX form parsed → BCP-47), and with `--json` prints `serde_json::to_string_pretty(&DiskUsageResponse)` whose `summary` already nests `{code, params}`.
- `tests/rest_disk_test.rs`: four tests — en-US round-trip with `?render=server` asserts EN catalogue opening, es-AR round-trip asserts Spanish opening, default off-by-default keeps raw Diagnostic (no `rendered_summary`, `summary.code` starts `rubix.system.disk.`), unknown tool → 404. Drives the router via `tower::ServiceExt::oneshot` (no live TCP).
- `tests/cli_disk_test.rs`: four tests — `LANG=en_US.UTF-8` and `LANG=es_AR.UTF-8` invocations of `CARGO_BIN_EXE_rubix-admin system disk` assert catalogue-matching prose; `--json` parses and asserts `summary.code` + `summary.params`; a grep guard walks `src/bin/rubix_admin/**.rs` for `reqwest`, `hyper::Client`, `TcpStream` and fails on regression.
- `cargo test -p rubix-agent` green across `authz_gate_test`, `cli_disk_test`, `mcp_disk_test`, `rest_disk_test` plus the inline unit tests in `routes::tools` (variant→status mapping + summary-render-by-language).
- Committed as `stage 3 (block 3, PR 5) — REST + CLI parity (final smoke seam)` (e0bb855).

## Next

- Stage 6 (REVIEW gate for stage 3) — fresh session reviews the diff against the SCOPE thin-slice exit criteria and runs the six-step manual smoke in `rubix/docs/scope/THIN-SLICE.md §Success criterion` end to end.

## What you need to know

- Handler body is 17 lines (`awk '/^async fn dispatch\(/,/^}$/' rubix/crates/rubix-agent/src/routes/tools.rs | wc -l`); shaping logic lives in the helper `shape_response` + `render_summary` to keep dispatch tight.
- `starter-i18n` feature `routes` was added to `rubix-agent`'s dep so `middleware::{accept_language_layer, LocaleCtx}` are visible. `serde` was promoted from dev-dep to runtime dep for the `#[derive(Deserialize)]` on `RenderQuery`.
- Re-uses `crate::boot::mcp::prefs_from_locale` for the REST + CLI `ResolvedPreferences` mapping — same en-US / es-AR table the MCP path uses. No new locale-preferences home was introduced.
- The CLI uses `unsafe { std::env::set_var(...) }` in its inline tests because Rust 1.90 (the toolchain on the sandbox) requires it; the project MSRV is 1.80, but `unsafe` blocks compile cleanly on both.
- `./rubix/scripts/lint-doc-refs.sh` is clean. Code-side doc links go to `docs/design/*/README.md` only.

## Open questions

- (none)
