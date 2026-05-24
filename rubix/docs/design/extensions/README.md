# Extensions — bootflow, REST surface, persistence, and the 10-minute scaffold

> **Authoritative framework:** the
> [`starter-extensions/`](../../../../starter-extensions/) sibling
> workspace. This page covers how `rubix-agent` *consumes* that
> framework; the framework's own design lives in
> `starter-extensions/DOCS/extensions/`. The dependency arrow only
> ever points rubix → starter-extensions (SCOPE R2).

`rubix-agent` discovers extensions on disk at boot, supervises their
processes, exposes a REST + SSE admin surface under
`/api/v1/extensions/*`, persists enable/disable state in Postgres, and
streams contributed UI panels into the Module-Federation host that
`packages/test-ui-5` mounts. Five upstream crates plus a thin
boot-time composer in `rubix-agent` make that work; the contributor
flow at the bottom of this page is what a fresh extension author
follows to land their own.

## Bootflow

`rubix-agent`'s boot composes seven upstream primitives in a single
linear async fn,
[`boot/extensions.rs::build_extension_admin`](../../../crates/rubix-agent/src/boot/extensions.rs):

1. **Migrate** — `PgEnablementStore` applies its own migration
   ([`starter-extensions/crates/starter-ext-store-pg/src/migrations/0001_extensions_enablement.sql`](../../../../starter-extensions/crates/starter-ext-store-pg/src/migrations/0001_extensions_enablement.sql))
   against the existing rubix PG pool. The SQL ships under the
   `starter-ext-store-pg` crate root, so rubix's own migration
   numbering and the upstream extensions migration numbering are
   independent (SCOPE OQ-6 resolution).
2. **Scan** — `starter_ext_host::Loader::scan(cfg.extensions.dir)`
   walks the immediate child directories looking for `block.yaml`,
   then `validate_all` checks the manifest shape before any state is
   committed (two-phase loader).
3. **Commit** — surviving records land in a sealed
   `ExtensionRegistry`.
4. **Read persisted state** — `PgEnablementStore::list_all()` returns
   `(ExtensionId, EnablementState)` pairs; rows missing from PG default
   to `Enabled` when `cfg.extensions.autostart_enabled_records = true`
   (the dev default), or `Disabled` otherwise.
5. **Spawn supervisors** — for each `Enabled` process-flavour record,
   `DefaultSupervisorFactory::spawn(record)` runs the binary under
   `starter-ext-supervisor`'s restart-with-backoff policy.
6. **Materialise admin handle** — `ExtensionAdmin::new(registry,
   supervisors, store, factory)` returns a cheap-to-clone handle the
   admin router hands every request handler.
7. **Log** — `INFO rubix.boot.extensions loaded=N autostarted=M` is
   the line operators grep for at startup.

Autostart audit rows are written under the synthetic principal
`system:extensions-autostart` (SCOPE OQ-5) so operators can tell a
boot-time replay apart from an operator-issued enable.

## REST surface

`rubix-agent::main` merges `starter_ext_server::router(admin)` under
`/api/v1/extensions/*` into its main `axum::Router`. The
`authz_gate` middleware applies — every lifecycle endpoint requires
`Role::Admin`. The bundle-serving routes (`/ui/*path`, `/i18n/:lang`)
are deliberately unauthenticated so the frontend host can fetch
public assets without a CSRF dance.

| Method | Path | Auth | Body / Returns |
|---|---|---|---|
| `GET` | `/api/v1/extensions` | Admin | List of records `{ id, version, state, supervisor_state, contributes }` |
| `GET` | `/api/v1/extensions/:id` | Admin | Full record incl. manifest |
| `POST` | `/api/v1/extensions/:id/start` | Admin | Drive supervisor to `Running` |
| `POST` | `/api/v1/extensions/:id/stop` | Admin | Drive supervisor to `Stopped` |
| `POST` | `/api/v1/extensions/:id/restart` | Admin | Stop + start |
| `POST` | `/api/v1/extensions/:id/enable` | Admin | Persist `Enabled` in PG, ensure supervisor up |
| `POST` | `/api/v1/extensions/:id/disable` | Admin | Persist `Disabled` in PG, drop supervisor |
| `GET` | `/api/v1/extensions/:id/events` | Admin | SSE — `lifecycle` / `log` / `error` events |
| `POST` | `/api/v1/extensions/install` | Admin | Multipart tarball → unpack, validate, reload (registry-URL path stubs `not_implemented`) |
| `DELETE` | `/api/v1/extensions/:id` | Admin | Stop, remove directory, mark PG row `Disabled` |
| `GET` | `/api/v1/extensions/:id/ui/*path` | (unauthed) | Federated bundle assets |
| `GET` | `/api/v1/extensions/:id/i18n/:lang` | (unauthed) | Catalogue fragment for the bundle |

All lifecycle writes audit a `changelog` row (`kind =
extension.<action>`) through the existing
`changelog_middleware`; install/uninstall write rows too but are not
registered as `Reversible` (installs typically happen outside the
agent loop — `rubix.undo.last` does not roll them back).

SCOPE OQ-2 resolution: the SSE event stream is mounted under the same
router as the lifecycle endpoints (`GET /:id/events`), not a separate
function — one merge call covers both.

## PG persistence

Enable/disable state survives restart because `PgEnablementStore`
upserts into `extensions_enablement(extension_id, state, updated_at,
updated_by)`. The `updated_by` column threads through from
`set_as(actor_id, id, state)` so audits are honest even when a
restart replays the table.

The migration namespacing is owned by `starter-ext-store-pg` itself —
rubix-agent applies it on boot but does not number it in its own
`migrations/` tree. This keeps the upstream crate self-contained and
re-usable by future consumers (insights-iot, insights-energy, any
in-tree starter sample) without coupling their migration order to
rubix's.

## The 10-minute scaffold

A fresh contributor starting from zero reaches a running, supervised,
REST-controllable, browser-renderable extension in well under ten
minutes:

1. **Copy** [`rubix/extensions/com.rubix.example/`](../../../extensions/com.rubix.example/)
   to `rubix/extensions/com.<org>.<name>/`.
2. **Edit** `Cargo.toml`:
   - rename `[package].name` (kebab-case),
   - rename `[[bin]].name` to match,
   - keep the `starter-ext-sdk` path dep (four `..` segments up to the
     repo root, then into
     `starter-extensions/crates/starter-ext-sdk`),
   - pick the SDK feature for your flavour (`process`, `inproc`, or
     `wasm`; process is the recommended starting flavour).
3. **Register** the new crate in
   [`rubix/extensions/Cargo.toml`](../../../extensions/Cargo.toml)
   `members` array.
4. **Edit `block.yaml`** — set `id`, `version`, and which of
   `tools` / `skills` / `flows` / `ui` you contribute. Tools point at
   a handler ident exported from your binary; skills point at a
   directory of `SKILL.md` files (quarantined by default per starter
   agent SCOPE R4); flows point at YAML rooted at `ai-agent`.
5. **Implement** `src/main.rs` against `starter-ext-sdk`. The macros
   (`#[derive(Extension)]`, `requires!{}`) generate the dispatch
   table; you write one `Tool` impl per contribution.
6. **Build**:
   ```bash
   cargo build --manifest-path rubix/extensions/Cargo.toml -p <crate>
   ```
7. **Boot `rubix-agent`.** The host loader picks the new bundle up
   from `cfg.extensions.dir` (`rubix/extensions` by default in dev).
   The boot log line names it; `GET /api/v1/extensions` returns it;
   `POST /api/v1/extensions/<id>/start` drives the supervisor; the
   contributed tool appears in `tools/list`; the contributed flow
   auto-surfaces over MCP via `FlowAsTool`; if the bundle exposes a
   UI panel, `packages/test-ui-5`'s `/extensions` route renders it.

Per SCOPE R8 your extension never depends on `rubix-domain`,
`rubix-tools`, `rubix-flows`, `rubix-skills`, `rubix-agent`, or any
`starter/crates/*` directly — only on `starter-ext-sdk`. Verified at
build time by the `rubix/extensions/Cargo.toml` workspace boundary.

## Frontend wiring

The `starter-ext-ui` Module-Federation host renders extension-supplied
React panels into named slots. `packages/test-ui-5` mounts an
`ExtensionHostProvider` configured with the rubix-agent base URL,
then drops `<ExtensionSlot id="main">` into its `/extensions` route.
The host queries `GET /api/v1/extensions/:id` to read the manifest,
fetches `GET /api/v1/extensions/:id/ui/remoteEntry.js`, and mounts
the exposed module bound to that slot (see
[`packages/test-ui-5/src/app/extensions/page.tsx`](../../../../packages/test-ui-5/src/app/extensions/page.tsx)).

`com.rubix.example` ships a minimal `ui/main.tsx` that renders
"hello from com.rubix.example v0.1.0" plus a host-context timestamp,
so the round-trip is observable without authoring a non-trivial
panel.

## `block.yaml` shape

The full manifest schema lives upstream at
[`starter-extensions/crates/starter-ext-spi/src/manifest.rs`](../../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs);
the example uses every supported `contributes.*` key:

```yaml
id: com.rubix.example
version: 0.1.0
runtime:
  kind: process
  bin: rubix-example-extension
contributes:
  tools:
    - { id: com.rubix.example.echo, handler: EchoTool }
  skills:
    - { dir: skills/ }
  flows:
    - id: com.rubix.example.assistant
      flow_file: flows/example-assistant.yaml
      auth: { require_role: Reader }
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: Main, module: "./main", slot: main }
```

`runtime.kind` is `process`, `inproc`, or `wasm`; rubix wires
`process` today (the other two are upstream-validated but not yet
exercised by a real rubix extension). `contributes.ui.entry` resolves
to the federation bundle the frontend host fetches.

## Trust + content-hash quarantine

Per starter agent SCOPE R4: extension-shipped skills default to
`trust: quarantined`. An operator approves the bundle by its
content hash; one byte changes → re-quarantined. This is the
single most load-bearing safety property and applies unchanged here —
the host loader hands every skill it discovers to the existing
skill-trust gate before the agent ever sees it.

## See also

- [`rubix/extensions/README.md`](../../../extensions/README.md) —
  how to add and build extensions in this workspace.
- [`docs/sessions/2026-05-24-extensions-wired.md`](../../sessions/2026-05-24-extensions-wired.md)
  — closing session note for the wire job, with the operator-runnable
  manual flow.
- [`starter-extensions/DOCS/extensions/scope/SCOPE.md`](../../../../starter-extensions/DOCS/extensions/scope/SCOPE.md)
  — the upstream framework's authoritative scope.
