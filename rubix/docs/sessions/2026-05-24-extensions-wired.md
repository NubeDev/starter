# 2026-05-24 — Extensions wired end-to-end + `starter-ext-store-pg` upstream

Closing session note for branch `codeless/rubix-extensions-wire`.
After this branch `rubix-agent` discovers, supervises, and serves
extensions through the upstream `starter-extensions` framework
end-to-end: boot loads bundles via `starter-ext-host`, supervises
process-flavour extensions via `starter-ext-supervisor`, exposes the
admin REST + SSE surface from `starter-ext-server` under
`/api/v1/extensions/*`, persists enable/disable in Postgres via the
new upstream `starter-ext-store-pg` crate, the reference extension
`com.rubix.example` builds and loads cleanly, and `packages/test-ui-5`
renders an extension-contributed React panel via the existing
`starter-ext-ui` Module-Federation runtime.

This was a wiring job, not an authoring one — every primitive
already existed upstream. The single new upstream piece is the
PG-backed `EnablementStore` impl, landed in Phase A per SCOPE R2 so
every future starter consumer benefits.

## Phases — what landed where

### Phase A — upstream: `starter-ext-store-pg`

- **A.1 — Crate scaffold.** New crate
  [`starter-extensions/crates/starter-ext-store-pg/`](../../../../starter-extensions/crates/starter-ext-store-pg/)
  with `PgEnablementStore { pool: PgPool }` impl `EnablementStore`,
  `get` / `set` / `set_as` / `list_all` (idempotent UPSERT on
  `extension_id`), and `migrations/0001_extensions_enablement.sql`
  defining `extensions_enablement(extension_id PK, state CHECK
  IN ('enabled','disabled'), updated_at, updated_by)`. Added to
  `starter-extensions/Cargo.toml` `[workspace.members]`. Commit
  `1752fac`.
- **A.2 — Testcontainers PG test + SCOPE update.**
  [`tests/store_test.rs`](../../../../starter-extensions/crates/starter-ext-store-pg/tests/store_test.rs)
  asserts roundtrip + UPSERT (set twice same id different state) +
  `list_all` ordering + `updated_by` audit thread. SCOPE entry
  appended naming the impl. Commit `7daaa72`.

### Phase B — `rubix/extensions/` workspace + reference build

- **B.1 — Sibling workspace bootstrap.** Added
  [`rubix/extensions/Cargo.toml`](../../../extensions/Cargo.toml) as
  its own `[workspace]` listing
  `com.rubix.example/process` as the sole member; verified the
  cross-workspace `starter-ext-sdk` path dep (four `..` up, then into
  `starter-extensions/crates/starter-ext-sdk`); confirmed
  `cargo build --manifest-path rubix/extensions/Cargo.toml -p
  com.rubix.example` produces `target/debug/rubix-example-extension`
  and the binary is invokable. SCOPE R8 verified at the workspace
  boundary — no `rubix-*` path-deps. Commit `853fc41`.
- **B.2 — README + CI build.**
  [`rubix/extensions/README.md`](../../../extensions/README.md)
  rewritten present-tense covering the layout, how to add a new
  extension, and how `rubix-agent` picks it up. Root
  `.github/workflows/ci.yml` learned a dedicated `rubix-extensions`
  job invoking `cargo build` against the sibling manifest. Commit
  `c2f7ab5`.

### Phase C — rubix-agent boot wiring

- **C.1 — `boot/extensions.rs` verb.** New
  [`rubix/crates/rubix-agent/src/boot/extensions.rs`](../../../crates/rubix-agent/src/boot/extensions.rs)
  exposes `build_extension_admin(cfg, pg_pool) -> Result<ExtensionAdmin,
  BootError>` composing `PgEnablementStore` + `Loader::{scan,
  validate_all, commit}` + `DefaultSupervisorFactory` + `ExtensionAdmin`,
  plus the `SYSTEM_AUTOSTART_PRINCIPAL` constant.
  [`boot/config.rs`](../../../crates/rubix-agent/src/boot/config.rs)
  grew an `[extensions]` section (`enabled` default true, `dir`
  default `rubix/extensions`, `autostart_enabled_records` default
  true); [`rubix/dev/agent.toml`](../../../dev/agent.toml) carries
  the dev block. Commit `578b4f7`.
- **C.2 — `main.rs` router + autostart.**
  [`rubix-agent::main`](../../../crates/rubix-agent/src/main.rs)
  calls `build_extension_admin` after the PG pool, merges
  `starter_ext_server::router(admin)` under `/api/v1/extensions/*`
  into the rubix-agent router under the existing `authz_gate`
  middleware, and emits `INFO rubix.boot.extensions loaded=N
  autostarted=M` at startup. Commit `51b0a7c`.
- **C.3 — MCP surface + changelog actor.** Confirmed SCOPE OQ-4 —
  `tools/list` includes `com.rubix.example.echo` post-wire because
  `starter-ext-mcp`'s adapter routes contributed tools into the same
  `ToolRegistry` `boot::mcp::build_mcp_surface` consumes. Confirmed
  SCOPE OQ-5 — autostart audits use the synthetic
  `system:extensions-autostart` principal so changelog rows
  distinguish boot replay from operator action. Commit `423b4b7`.

### Phase D — install/uninstall + frontend

- **D.1 — Install/uninstall endpoints.** Added `POST
  /api/v1/extensions/install` (multipart tarball; registry-URL path
  returns a stubbed `not_implemented` Diagnostic) and `DELETE
  /api/v1/extensions/:id` (stop supervisor, remove directory, mark
  PG row `Disabled`). Four new MessageKeys
  (`rubix.extension.install.{succeeded,invalid_manifest}`,
  `rubix.extension.uninstall.{succeeded,not_found}`) in `en.json` +
  `es.json` same commit. Integration test asserts a tarball
  roundtrip. Commit `11bf66b`.
- **D.2 — `test-ui-5` `ExtensionHostProvider` page.**
  [`packages/test-ui-5/src/app/extensions/page.tsx`](../../../../packages/test-ui-5/src/app/extensions/page.tsx)
  wires `ExtensionHostProvider` with the rubix-agent base URL and
  renders `<ExtensionSlot id="main">` visibly. The example
  extension gained a minimal `ui/main.tsx` exporting a React
  component that renders `hello-from-com.rubix.example` with the
  host-context theme; `block.yaml`'s `contributes.ui.exposes` names
  the `Main` module against the `main` slot. `pnpm --filter
  @nube/test-ui-5 typecheck + test + lint` green. Commit `d70c824`.

### Phase E — integration test + closing docs

- **E.1 — Full lifecycle integration test.**
  [`rubix/crates/rubix-agent/tests/extensions_lifecycle_test.rs`](../../../crates/rubix-agent/tests/extensions_lifecycle_test.rs)
  boots against testcontainers PG with a fixture extensions dir
  containing `com.rubix.example`; asserts all eight SCOPE Phase E
  contracts: `GET` lists the example, `start` transitions to
  `Running`, `stop` to `Stopped`, `restart` cycles cleanly,
  `disable` then restart loses the supervisor, `enable` brings it
  back, the PG row reflects the final state, and the event stream
  emits lifecycle messages in order. Commit `508f71f`.
- **E.2 — Closing docs + this note.**
  [`docs/design/extensions/README.md`](../design/extensions/README.md)
  rewritten present-tense covering bootflow, REST surface, PG
  persistence, the 10-minute scaffold (now executable), frontend
  wiring, and the `block.yaml` shape — stale "planned upstream"
  pointers removed. THIN-SLICE.md "Goals lit up beyond the thin
  slice" table gained an Extensions-wired row.

## Operator-runnable manual flow

The same path is reachable without the test harness. Boot
`rubix-agent` against a fresh PG (e.g. `mani run run`); then from
the host:

```bash
# 1. List discovered extensions — the boot log already printed the count.
curl -b cookies.txt http://127.0.0.1:8088/api/v1/extensions
# → [{ "id": "com.rubix.example", "version": "0.1.0",
#      "state": "enabled", "supervisor_state": "Running", ... }]

# 2. Stop the supervisor, observe the lifecycle change.
curl -b cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/extensions/com.rubix.example/stop
curl -b cookies.txt http://127.0.0.1:8088/api/v1/extensions/com.rubix.example
# → "supervisor_state": "Stopped"

# 3. Disable persists in PG; subsequent restarts will not bring it back.
curl -b cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/extensions/com.rubix.example/disable
psql -d rubix -c "SELECT extension_id, state, updated_by
                  FROM extensions_enablement;"
# → ('com.rubix.example', 'disabled', '<your-operator-id>')

# 4. Re-enable; supervisor starts back up.
curl -b cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/extensions/com.rubix.example/enable

# 5. Tail the lifecycle event stream.
curl -N -b cookies.txt \
  http://127.0.0.1:8088/api/v1/extensions/com.rubix.example/events
# → SSE: { "kind": "lifecycle", "from": "Stopped", "to": "Running" } ...

# 6. Confirm the contributed tool surfaces over MCP.
curl -b cookies.txt http://127.0.0.1:8088/api/v1/tools \
  | jq '.[] | select(.id == "com.rubix.example.echo")'

# 7. Browse to test-ui-5's /extensions route and see the panel render.
#    The host fetches /api/v1/extensions/com.rubix.example/ui/remoteEntry.js,
#    mounts the federated module into the `main` slot, and the panel
#    prints "hello from com.rubix.example v0.1.0".
```

## Evidence summary

- **cargo:** `cargo build --workspace` green across both the rubix
  workspace and the `rubix/extensions` sibling workspace; same for
  the `starter-extensions` workspace (the new `starter-ext-store-pg`
  crate inclusive).
- **Rust tests:** `cargo test -p starter-ext-store-pg --features
  testcontainers` (Phase A.2) and `cargo test -p rubix-agent --test
  extensions_lifecycle_test` (Phase E.1) — both pass under
  testcontainers PG.
- **Vitest:** `pnpm --filter @nube/test-ui-5 test` covers the
  `/extensions` page's slot mounting (D.2).
- **CI:** `.github/workflows/ci.yml` runs `cargo build` against
  `rubix/extensions/Cargo.toml` on every PR (B.2).
- **Boot log line:** `INFO rubix.boot.extensions loaded=1
  autostarted=1` after Phase C.2.
- **Changelog actors:** autostart uses
  `system:extensions-autostart`; operator-driven enable/disable use
  the request subject id via `set_as`.

## Open questions resolved

- **OQ-1 — Default for `cfg.extensions.dir`.** Dev:
  `rubix/extensions`. Prod: documented as `/var/lib/rubix/extensions`
  in `rubix/dev/agent.toml`'s prod profile comment.
- **OQ-2 — SSE event-stream in the same router?** Yes — Phase C.1
  confirmed `starter_ext_server::router(admin)` already mounts
  `GET /:id/events` alongside the lifecycle endpoints; one merge
  call covers both.
- **OQ-3 — Does the example ship UI assets?** No, originally;
  Phase D.2 added a minimal `ui/main.tsx` plus `block.yaml`
  `contributes.ui.exposes`.
- **OQ-4 — Does the example's `echo` tool auto-surface over MCP?**
  Yes — `starter-ext-mcp`'s adapter routes contributed tools into
  the shared `ToolRegistry` `boot::mcp` already consumes. Verified
  at C.3.
- **OQ-5 — Changelog actor for autostart-on-boot.**
  `system:extensions-autostart` — defined as the
  `SYSTEM_AUTOSTART_PRINCIPAL` const in `boot/extensions.rs`.
- **OQ-6 — Migration namespacing collision.** No collision —
  `starter-ext-store-pg` owns its own migration root; rubix-agent
  applies it via `include_str!` but does not number it under
  rubix's own `migrations/` tree. The two migration sets are
  independent.

## After this branch

- A registry server for `POST /install` is still the operational
  gap — today operators upload tarballs directly. The
  registry-URL path stubs `not_implemented` for now.
- WASM and inproc flavours are upstream-validated but no rubix
  extension exercises them yet; a follow-up job lands once we
  author one.
- Per-tenant extension scoping and finer-grained lifecycle
  AuthZ remain explicitly out of scope per SCOPE.
