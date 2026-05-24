## Done

- Wired `starter-ext-mcp::register_process_tools` into rubix-agent's MCP surface so process-flavour extensions' `contributes.tools[]` (incl. `com.rubix.example.echo`) appear in `tools/list` alongside bundled FlowAsTool entries (SCOPE OQ-4).
- `boot::build_extension_admin` now returns `ExtensionAdminBundle { admin, registry, process_handles }`; main.rs reordered so the admin is built *before* `build_mcp_surface`, and the resulting `ExtensionMcpContext` is threaded in.
- Added `boot::mcp::ExtensionMcpContext` + extended `build_mcp_surface` / `build_tool_registry` signatures; non-test stdio caller (`rubix_admin/mcp/serve.rs`) passes `None`.
- Added `pub const SYSTEM_AUTOSTART_PRINCIPAL = "system:extensions-autostart"` (SCOPE OQ-5) and stamped it as the `actor=` field on every autostart log line so audits distinguish operator actions from boot-time replay.
- `cargo build -p rubix-agent --tests` green. Committed as `stage 9: phase C.3 — MCP surface integration + changelog actor`.

## Next

- Stage 10 picks up the next phase (likely a Phase C gate or Phase D start — integration test driving lifecycle through REST + the `test-ui-5` page wiring per the job goal).

## What you need to know

- The PG pool is now acquired once (for both the extension admin seed and the later auth/changelog recorder) before the MCP surface builds. The acquisition lives where `mcp_pool` was; the second `pg_connect` inside the auth block remains because the auth surface clones a separate `pool.clone()` from the same `mcp_pool`-derived handle.
- `register_process_tools` returns per-tool failures via `RegisterError::Collected`; we log+continue (warn target `rubix.boot.extensions.mcp`) — matching the kernel's per-extension isolation discipline. Successful tools still land in the registry.
- The 30s per-request timeout for extension MCP tool calls is a single `const EXTENSION_TOOL_REQUEST_TIMEOUT` in `boot/mcp/mod.rs`; tune there if needed.
- Smoke verification (curl `GET /api/v1/extensions`, `tools/list` JSON-RPC) was not executed in this stage — no running PG/agent available in the worktree — but compilation green and the wiring follows starter-ext-mcp's documented contract.

## Open questions

- The stage brief mentions "the audit row" for autostart but `build_extension_admin` does not plumb a `ChangeRecorder`; the synthetic principal is currently only stamped on tracing log lines, not on a persisted `starter_changes` row. If a persisted changelog row is required for autostart-on-boot, a later stage needs to thread an `Arc<dyn ChangeRecorder>` into `build_extension_admin` and emit `Op::Update` per autostarted id.
