## Done

- Reviewed Phase C diff (commits 578b4f7 C.1, 51b0a7c C.2, 423b4b7 C.3) against Layer-1 invariants
- Confirmed R1 dep direction: rubix-agent pulls starter-ext-{spi,host,server,supervisor,store-pg}, no reverse edges
- Confirmed R2 single transport: SSE handler `events::events` mounted on the same `starter-ext-server` Router at `/extensions/:id/events`, no parallel router
- Confirmed R4/R5: lifecycle routes wrapped by `with_principal + with_role(Role::Admin)`; UI/i18n catalogue intentionally unauthed
- Confirmed wire-formats untouched (no proto/openapi diff in C.1–C.3)
- PASS: Layer-1 invariants hold across Phase C — single router, correct dep direction, admin sandwich on lifecycle, MCP tools merged before Arc seal.

## Next

- Phase D (UI host wiring + test-ui-5 panel + integration test through REST) per SCOPE — a fresh session picks it up

## What you need to know

- Three Phase C work commits: 578b4f7 (boot/extensions.rs verb file + config + agent.toml), 51b0a7c (main.rs router mount + boot summary line), 423b4b7 (MCP auto-surface + SYSTEM_AUTOSTART_PRINCIPAL)
- Expected tools/list contains `com.rubix.example.echo` via `starter_ext_mcp::register_process_tools` merged into ToolRegistry pre-Arc-seal (C.3 main.rs reorder)
- GET /api/v1/extensions served by `starter_ext_server::router_with_auth(admin, authenticator)` nested under `/api/v1` (C.2 main.rs); list/detail/events/enable/disable all under that single router
- Boot log: single INFO summary `rubix.boot.extensions loaded=N failed=N autostarted=N` from `build_extension_admin` plus per-step lines on `rubix-agent::boot::extensions` target (scanning root, registry sealed, autostarted supervisor)
- SCOPE OQ-2 evidence: SSE event stream is in the SAME router — `starter-extensions/crates/starter-ext-server/src/router.rs:78` registers `/extensions/{id}/events` via `get(events)` from `events.rs` on the same `Router` as list/detail/toggle; not a separate transport
- This is a gate stage — no code edits made, no commit needed; nothing to stage
- PASS: Layer-1 invariants hold — single router (incl. SSE), correct crate dep direction, admin sandwich on lifecycle, MCP tools merged before registry seal, wire-formats untouched.

## Open questions

- (none)
