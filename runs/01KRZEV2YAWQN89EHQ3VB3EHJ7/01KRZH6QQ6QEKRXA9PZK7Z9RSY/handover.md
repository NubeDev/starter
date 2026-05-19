## Done

- crates/starter-jsonrpc-stdio: new parent-workspace crate with Content-Length-framed async JSON-RPC reader/writer; 8 unit tests; replaces starter-mcp's newline framing
- starter-extensions/crates/starter-ext-supervisor: full crate (lib.rs barrel + backoff, restart, event_ring, handshake, capability, stream, supervisor modules); 20 unit tests; tests/hello_process.rs integration test passes end-to-end
- starter-ext-sdk process flavour body: run_process_main + register_process_main! macro; init handshake with manifest hash, health responses, tools/<id> dispatch, shutdown
- examples/hello-process: same source as hello-builtin (trait impl byte-identical), block.yaml with runtime.kind=process, builds and runs
- starter-mcp/src/server/stdio_loop.rs swapped to Content-Length framing via the new crate (existing 7 dispatch tests still pass)
- committed as bcb0e54

## Next

- Stage 8: Kernel Phase 2 wiring — starter-ext-server admin routes (GET /extensions, GET /extensions/<id>/events, POST .../enable|disable) that surface the SupervisorHandle's state watch, event ring, and capability_violation counter. Then the post-R13 adapter phases.

## What you need to know

- cargo build --workspace inside starter-extensions FAILS BY DESIGN: hello-builtin and hello-process each select a mutually-exclusive cargo feature on starter-ext-sdk and cargo unifies features across the workspace, which trips R1's duplicate-#[no_mangle] linker error. Build per-package (cargo build -p hello-builtin / -p hello-process) — that is the SCOPE-prescribed workflow.
- Integration test target/debug/hello-process must exist before `cargo test -p starter-ext-supervisor --test hello_process` runs; the test skips itself with a clear message otherwise.
- SIGTERM-first shutdown is implemented as "tokio start_kill at the grace deadline" on every platform in v0.1 — std has no public SIGTERM sender and SCOPE allowed pulling nix later. The grace window behaviour (SIGTERM → wait → SIGKILL) is what callers observe, but the polite signal is currently SIGKILL on every platform. Documented inline at supervisor::send_sigterm.
- run_process_main does not yet invoke ExtensionBehavior::on_init / on_shutdown — the existing hello-builtin example does not implement ExtensionBehavior (only the proc-macro-generated Handlers trait), and R1 forbids per-flavour deltas in the trait surface. The supervisor's `shutdown` notification still exits the loop cleanly. Wiring lifecycle hooks lands additively alongside a unified ExtensionBehavior requirement for hello-builtin.
- Capability gate is advisory: gate.check refuses ungranted host methods at the wire boundary and increments the counter, but the host-side method bodies themselves are stubbed in v0.1 (every allowed call returns Error::ExtensionInternal "not implemented"). The supervisor's wire shape is correct; the backends fill in with the adapter phases.

## Open questions

- (none)
