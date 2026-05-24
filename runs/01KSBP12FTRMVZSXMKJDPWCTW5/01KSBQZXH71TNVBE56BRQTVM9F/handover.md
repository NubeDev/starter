## Done

- Added `rubix-admin mcp` subcommand: `bin/rubix_admin/mcp/{mod.rs,serve.rs}`, registered in `main.rs` (Mcp variant, tracing init skipped for the verb so stdout stays clean for MCP framing).
- Extracted `boot::mcp::build_tool_registry()` shared by HTTP `build_mcp_surface` and the new stdio path.
- Principal resolution: `RUBIX_PRINCIPAL_EMAIL` → `PgUserStore::find_by_email` when `database_url` set; localised stderr `Diagnostic` + exit 1 on missing/unknown; no-DSN tolerance synthesises principal (matches HTTP binary).
- Locale cascade: per-call `_meta.acceptLanguage` (U1) > POSIX `LANG` parsed via `parse_lang_env` > `"en"` (outer `with_locale` wraps the loop).
- Added `rubix.admin.mcp.principal.{missing,not_found}` catalogue keys in EN + ES.
- Added `rubix/dev/claude-desktop.example.json`, the `mcp-stdio` mani task.
- Updated `docs/design/agent/README.md` (new "stdio MCP transport" section) and `docs/design/i18n-prefs/README.md` (new three-step stdio locale-cascade subsection).
- Integration test `tests/mcp_stdio_test.rs` spawns the binary, drives initialize + tools/list + tools/call for `en-US` and `es-AR`, and asserts non-zero localised-stderr exit when `RUBIX_PRINCIPAL_EMAIL` is missing. DB-backed unknown-principal test is `#[ignore]`.
- `cargo build`, `cargo test -p rubix-agent`, `./rubix/scripts/lint-doc-refs.sh`, `python3 -m json.tool rubix/dev/claude-desktop.example.json` all green.
- Committed as `565d54f` on `codeless/rubix-demo-wiring`.

## Next

- Stage 6 (REVIEW gate for block C) is next per the three-block plan.

## What you need to know

- `starter_observability::tracing::init` writes to stdout — that's why the `mcp` verb opts out of the global subscriber and routes operator messages through `eprintln!`. Don't reintroduce it without redirecting to stderr.
- `starter_mcp::run_stdio` consumes `ToolRegistry` by value, so `build_tool_registry` returns a fresh registry; the HTTP path wraps it in `Arc` afterward.
- The `mcp_stdio_test` default tests run the binary with `RUBIX_DATABASE_URL` removed from the env to exercise the no-DSN synthetic-principal branch — that's how they get to `tools/list` and `tools/call` without a real Postgres. The DB-backed unknown-principal assertion is gated on `RUBIX_TEST_DSN` and `#[ignore]`d, matching the existing `authz_gate_test` convention.
- `UserRecord` has no `disabled` field today, so the spec's "doesn't exist or is disabled" check is currently just the existence check. If a disabled flag lands upstream, extend `resolve_principal` accordingly.
- The synthetic-principal path warns to stderr in `en` ("rubix-admin mcp: database_url unset — synthesising..."). Don't be surprised by it in test stderr.

## Open questions

- (none)
