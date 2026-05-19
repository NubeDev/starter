## Done

- Added new crate `starter-extensions/crates/starter-ext-server` with the full Kernel Phase 2 admin slice: `ExtensionAdmin` state container + builder, `EnablementStore` trait + `InMemoryEnablementStore`, `SupervisorFactory` trait + `DefaultSupervisorFactory`, and `router` (unauthed, for TestApp) + `router_with_auth` (production, gates the privileged endpoints with `with_principal` + `with_role(Role::Admin)` from `starter-server`).
- Endpoints implemented: `GET /extensions`, `GET /extensions/{id}`, `GET /extensions/{id}/events` (paginated `{events, next_seq}` snapshot with `?after=&limit=` plus SSE upgrade on `Accept: text/event-stream` / `?stream=1`, polling at 250 ms), `POST /extensions/{id}/enable|disable` (store-first persistence; idempotent; calls `SupervisorFactory::spawn` / `handle.shutdown()`), `GET /extensions/{id}/ui/{*path}` (strong SHA-256 ETag cache keyed by path+mtime+size, `If-None-Match` → 304, `safe_join` rejects parent-traversal).
- Extended `EventRing` with a monotonic per-push `seq` field plus `since(after)` / `next_seq()` for SSE resume; unit test added.
- Workspace `starter-extensions/Cargo.toml` gained the new member, the parent's `starter-server` workspace dep, and aligned HTTP-stack deps (axum 0.8, tower 0.5, tower-http 0.6, http 1, futures 0.3).
- Added 7 end-to-end tests via `tower::ServiceExt::oneshot` (list / detail-404 / detail / enable+disable round-trip / events-404-when-no-supervisor / UI ETag+304 / UI traversal rejection) and one `safe_join` unit test. `cargo test -p starter-ext-server` and `cargo test -p starter-ext-supervisor` both green.
- Committed as `ce10e0f`.

## Next

- Stage 9 (next phase) — a fresh session will pick it up per the workflow.

## What you need to know

- The supervisor live-tail is *polling-based* (250 ms cadence over the `EventRing` snapshot). The doc comment in `events.rs` flags this and the v0.2 upgrade path (`tokio::sync::broadcast` in the supervisor) — wire shape is stable, only the backing mechanism changes.
- `router()` (no auth) is for TestApp only; production must use `router_with_auth(admin, Arc::new(authenticator))`. UI bundle path stays unauthed in both, by design (Module-Federation hosts load `remoteEntry.js` without credentials).
- The UI route is mounted as `/extensions/{id}/ui/{*path}` (axum 0.8 wildcard syntax) and uses a `UiParams` struct extractor — axum 0.8 didn't accept a `Path<(String, String)>` tuple here.
- Workspace-wide `cargo build --workspace` fails on a pre-existing duplicate `__STARTER_EXT_FLAVOUR_MARKER` in `starter-ext-sdk` (reproduces on a clean stash). Not introduced or touched by this stage.

## Open questions

- (none)
