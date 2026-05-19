## Done

- starter-ext-host crate: Loader::scan walks the extensions root one level deep, per-bundle errors never short-circuit; validate_all runs R4 namespace ownership + R6 capability compatibility + schema version + id-uniqueness checks; commit installs every record (Validated or Failed) atomically; ExtensionRegistry exposes list/get/state/iter_validated and is immutable after seal().
- starter-ext-mcp adapter: walks ExtensionRegistry.iter_validated(), looks each builtin extension up in starter_ext_sdk::builtin::BuiltinTable, builds one ExtensionToolBinding per contributes.tools entry (description + input_schema read from bundle files at load time, R7), registers each into starter_mcp::ToolRegistry via the existing Tool trait. Per-tool failures isolate as RegisterError::Collected.
- Phase-1 Ctx stub: deny-all backends for secrets/http_out/fs/wall_clock; tracing silently swallows; bounded mpsc event channel so streaming handlers compile. Real backends land in later phases.
- examples/hello-builtin: minimal builtin crate (Hello struct, requires![], one echo handler, register_static_table!) with block.yaml + schemas + docs. Compiles cleanly.
- End-to-end test (crates/starter-ext-mcp/tests/hello_builtin_end_to_end.rs): copies the bundle into a tempdir, runs the full kernel → adapter → ToolRegistry path, dispatches an `echo` call and verifies round-trip. Companion test stages a deliberately-broken sibling and asserts the bad-manifest-is-isolated smoke test from SCOPE.
- Workspace Cargo.toml now lists the three new members and exposes shared dep entries (starter-mcp via path = "../crates/starter-mcp", async-trait, tempfile).
- Commit f20a9a4 on codeless/starter-extensions.

## Next

- Stage 6: Kernel Phase 2 — starter-ext-supervisor (process flavour, stdio JSON-RPC, restart policy + intensity cap + backoff, health checks, event ring) plus starter-ext-server admin routes, with hello-process flipping one cargo feature on hello-builtin's source.

## What you need to know

- Dependency arrow deviation: starter-ext-mcp depends on starter-ext-sdk with `features = ["builtin"]` because BuiltinTable lives inside the SDK. starter-ext-host itself stays clean (ext-spi only). Stage 6+ adapters for process / wasm will pull in their respective per-flavour SDK feature the same way.
- Phase-1 CtxInner stub returns Error::Capability for every backend except tracing (no-op). Handlers that don't touch capabilities (like hello-builtin's echo) run cleanly; anything that calls ctx.http() / ctx.secrets() / etc. surfaces a clear "backend not wired in Phase 1" error.
- Loader::commit also stores Failed records (keyed `<unparsed:<dir-name>>` when the id never parsed). Adapters skip them by walking iter_validated().
- Manifest schema version check refuses anything other than v=1 with a typed "unsupported manifest schema" Manifest error, as decided in Stage 0.
- Capability-compatibility check only enforces `requires:` entries whose id starts with `cap.` (e.g. `cap.http_out`). Interface dependencies like `starter.spi.tool` are left to the host's interface registry (Stage 9-ish).
- Test invocation: `cargo test --manifest-path starter-extensions/Cargo.toml` from the worktree root; the sibling workspace is intentionally not in the parent's members list.

## Open questions

- (none)
