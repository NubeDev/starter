## Done

- Added `cleanup.rs` to `starter-ext-server`: `CleanupItem`, `CleanupKind` (WarehouseTable/EnablementRow/UiCache/I18nCache/Skill/Subscription), `CleanupError`, and the async `CleanupProvider` trait (`discover` + `purge`).
- Built-in providers (no rubix knowledge): `EnablementRowProvider` (DELETEs the row outright — added `EnablementStore::delete` with InMemory + Pg impls, killing the ghost row), `UiCacheProvider` / `I18nCacheProvider` (evict the ETag/byte caches per the extension's own path prefix; `EtagCache` gained `entries_under_prefix` + `evict_exact` and is now shared via `Arc`).
- `DELETE /extensions/{id}?purge=true` runs uninstall then every registered provider's purge (idempotent — already-uninstalled ids return `200 cleanup.succeeded` with whatever was removed, never 404; purge path skips the `set(Disabled)` so it doesn't resurrect the row). `?purge=false` keeps today's behaviour. Each purge step logs `target=starter_ext_server::cleanup` with the caller principal.
- `GET /extensions/{id}/cleanup` returns the dry-run manifest (`items` + `total_bytes`).
- Built-in providers auto-register at `ExtensionAdmin::build()`; consumers add more via `ExtensionAdminBuilder::with_cleanup_provider`.
- `restart_required` surfaced on the list projection (pending-restart set; install response gains `pending_restart: true`).
- Tests: per-provider discover/purge + namespace-scope (cleanup.rs unit tests), HTTP-level dry-run / purge / idempotent ghost-row / non-purge-keeps-Disabled (`tests/cleanup_routes.rs`). Touched crates build, clippy `-D warnings`, fmt, and tests all green.
- Committed as `79aaafe`.

## Next

- Stage 6 (rubix wiring): register `WarehouseCleanupProvider` + `SkillCleanupProvider` in `rubix/crates/rubix-agent/src/boot/extensions.rs` via `.with_cleanup_provider(...)`; project `issues`/`process`/`metrics`/cleanup into the admin envelope in `admin/extensions.rs`; add frontend tabs + uninstall dialog and the new client hooks.

## What you need to know

- `CleanupProvider::discover`/`purge` take `manifest: Option<&Manifest>` (not the scope sketch's `&Manifest`) so leftovers for a `Failed` (unparsed-manifest) record and ghost rows are still reachable — required by the idempotency contract.
- The full `cargo build --workspace` for `starter-extensions` fails in **pre-existing** ways I did not touch: `starter-ext-wasm` has a non-exhaustive `Capability` match (`Secrets`/`Custom` → needs a new arm), and building all examples together hits a `__STARTER_EXT_FLAVOUR_MARKER` link clash (builtin+wasm feature unification). Validate via per-crate `cargo test -p starter-ext-server -p starter-ext-store-pg`.
- There is a pre-existing user stash (`stash@{0}: user-WIP-aside-during-stage5-commit`) — left untouched.

## Open questions

- (none)
