## Done

- Added `rubix/crates/rubix-agent/src/boot/extensions.rs` with `build_extension_admin(cfg, pg_pool) -> Result<ExtensionAdmin, BootError>`: applies the `extensions_enablement` migration (via `include_str!` of the upstream SQL), constructs `PgEnablementStore`, runs the two-phase `Loader` + sealed `ExtensionRegistry`, autostarts supervisors for persisted-Enabled records via `DefaultSupervisorFactory`, and materialises `ExtensionAdmin` with that pre-populated supervisor map.
- Extended `boot/config.rs` with `ExtensionsConfig { enabled: bool=true, dir: PathBuf="rubix/extensions", autostart_enabled_records: bool=true }` plus default wiring; re-exported via `boot/mod.rs`.
- Added matching `[extensions]` block to `rubix/dev/agent.toml`.
- Added cross-workspace path deps in `rubix-agent/Cargo.toml` for starter-ext-{spi,host,server,supervisor,store-pg} + `thiserror`.
- `cargo build -p rubix-agent` green; committed as `578b4f7` with message starting "phase C.1 — boot/extensions.rs verb file".

## Next

- C.2: wire `build_extension_admin` into `rubix/crates/rubix-agent/src/main.rs` after the PG pool is built, merge `starter_ext_server::router(admin.clone(), ..)` under `/api/v1/extensions/*` with the existing `authz_gate` middleware; gate construction on `cfg.extensions.enabled`.
- C.3: MCP surface integration — feed extension-contributed tools into the same `ToolRegistry` `boot::mcp::build_mcp_surface` consumes; verify `system` actor used for autostart changelog rows (SCOPE OQ-5).

## What you need to know

- The migration SQL is pulled via `include_str!("../../../../../starter-extensions/crates/starter-ext-store-pg/src/migrations/0001_extensions_enablement.sql")` — relative path crosses the parent→sibling workspace boundary. If layout shifts, this constant breaks at compile time, not runtime.
- `BootError::AutostartSpawn` exists as a variant but is currently unreachable — spawn failures log at warn! and continue. Kept on the enum surface so C.2/E can promote to a hard fail if SCOPE demands it.
- `factory.rs` upstream exposes `SupervisorFactory` publicly but `DynFactory` is `pub(crate)`; we annotate the local binding as `Arc<dyn SupervisorFactory>` and the builder's `with_supervisor_factory(DynFactory)` accepts it via deref coercion.
- `Loader::scan` treats a missing extensions dir as an empty load, so `cargo run -p rubix-agent` boots cleanly on a checkout where `rubix/extensions/` has been deleted — no special handling needed at the boot layer.

## Open questions

- (none)
