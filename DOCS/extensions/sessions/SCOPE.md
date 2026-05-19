# extension hosting in a starter app — Scope

## One-line summary

A starter-based product that wants to load, serve, and surface
extensions imports `starter-ext-host` + the adapter crates it needs,
wires them into its existing `ServerBuilder`, and gets `/extensions/*`,
`/tools/*`, REST contributions, and Module-Federation UI serving
alongside its own routes — all behind the **same** `Authenticator` it
already uses.

## The problem this scope resolves

The `starter-extensions` workspace ships every adapter crate needed to
host extensions in a real product:

| Crate | What it surfaces |
|---|---|
| `starter-ext-host` | `Loader` + `ExtensionRegistry` |
| `starter-ext-server` | `GET/POST /extensions/*` admin slice, REST contributions |
| `starter-ext-mcp` | MCP tools at `POST /tools/<id>` |
| `starter-ext-cli` | CLI subcommands via `CommandRegistry` |
| `starter-ext-grpc` | gRPC backplane (`starter.ext.grpc.v1.ExtensionGrpc`) |
| `starter-ext-workers` | Periodic worker scheduler |

All of these are fully implemented and tested.

The reference consumer app — `examples/notes` — imports **none of
them**. Its `server.rs` wires `starter-auth-token`, `starter-mcp`,
and the consumer's own REST routes into `ServerBuilder`. The extension
substrate doesn't appear. The result is:

- `GET /extensions` → 404 on the notes server.
- The notes frontend cannot show loaded extensions.
- A developer reading `examples/notes` sees no example of how to turn
  a starter app into an extension host.
- `starter-extensions/examples/notes` (the extension-side example
  added in this session) runs on a *separate* process with no auth and
  no real server — proving the extension substrate works in isolation
  but not how it integrates with a full product.

This is a documentation and example gap, not a library gap. Every
library needed exists. The gap is that the canonical "how do you add
extension hosting to a starter app?" example has never been written.

## Why the gap happened

`starter-extensions` was developed as a sibling workspace to `starter`
(deliberately, per SCOPE.md "Relationship to starter"). The two
workspaces share only `starter-spi`. The `examples/notes` crate lives
in the `starter` workspace and was written before `starter-extensions`
reached the point where the integration was worth demonstrating.

The extension adapter crates (`starter-ext-server`, `starter-ext-grpc`,
etc.) live in `starter-extensions`, which means `examples/notes` has to
add a cross-workspace path dep to pull them in. That is intentional and
correct — it mirrors exactly what a third-party consumer does — but it
also means the integration was never written as part of either
workspace's normal development flow.

## What the fix looks like

### Phase 1 — wire the extension host into `examples/notes`

`examples/notes/src/server.rs` gains four additions, all additive:

1. **Loader at startup** — `Loader::scan(extensions_dir).validate_all()`
   + `Loader::commit` + `registry.seal()`. The extensions directory
   defaults to `./extensions/` beside the binary and is overridable via
   `EXTENSIONS_DIR`. An empty directory is valid — the server starts
   with zero extensions loaded.

2. **Admin slice** — `starter-ext-server::router_with_auth(admin,
   authenticator)` merged into `ServerBuilder`. This mounts:
   - `GET  /extensions` — list all loaded extensions + lifecycle state
   - `GET  /extensions/:id` — full manifest + capability grants
   - `GET  /extensions/:id/events` — stderr / lifecycle event stream
   - `POST /extensions/:id/enable` / `disable` — operator toggles
   - `GET  /extensions/:id/ui/*path` — serve the extension's UI bundle
   All five admin endpoints are gated `Role::Admin` by
   `router_with_auth`; the UI bundle path is deliberately unauthed (the
   federation runtime loads it with no credentials).

3. **REST + MCP adapters** — `rest_router` + `register_tools` merged
   into `ServerBuilder`. Extension-contributed REST routes appear
   alongside `/notes/*`; extension tools appear at `POST /tools/<id>`
   alongside the existing `NoteSearchTool`.

4. **gRPC backplane** — `extension_grpc_server` added to the tonic
   `Server` that already runs the `NoteService`. Extension-contributed
   gRPC RPCs appear as methods on
   `starter.ext.grpc.v1.ExtensionGrpc`; the existing `NoteService`
   is unchanged.

No existing route, auth check, or migration changes. The extension
substrate is purely additive.

### Phase 2 — add an Extensions panel to the frontend

`examples/notes/frontend/src/` gains `extensions-client.ts` (a thin
`StarterClient` wrapper for `GET /extensions/:id`) and
`ExtensionsView.tsx` (a tab alongside the notes list). The view shows:

- Each loaded extension as a card: id, version, display name, state
  badge (validated / failed), runtime kind (builtin / process / wasm).
- The full `contributes` breakdown: tools, REST routes, CLI commands,
  UI panels.
- An enable/disable toggle wired to `POST /extensions/:id/enable` and
  `POST /extensions/:id/disable`.

Auth is inherited — the same bearer the user logged in with is
forwarded to `/extensions/*`. The admin role gate in the server means
only admin tokens see these routes; non-admin users get 403 and the
tab is hidden.

### Phase 3 — mount the extension UI panel in the sidebar

`examples/notes/frontend/src/App.tsx` gains an
`<ExtensionSlot slot="sidebar" />` component from
`@nube/starter-ext-ui`. When a loaded extension contributes
`ui.exposes[].slot = "sidebar"`, its `remoteEntry.js` is fetched from
the server's `/extensions/:id/ui/*` path and the panel mounts beside
the notes list. This proves Module Federation round-trips end-to-end
in the context of a real product, not just a standalone example.

## Hard rules (load-bearing)

### R1 — one server, one auth surface

The extension admin routes are mounted inside the existing
`ServerBuilder` and gated by the existing `Authenticator`. There is no
second server, no second auth check, and no CORS exception for a
sidecar. The `starter-ext-server::router_with_auth` function takes the
same `Arc<dyn Authenticator>` the notes server already holds; no new
auth concept is introduced.

### R2 — the extension directory being empty is not an error

`Loader::scan` on an empty or missing directory returns zero candidates.
`Loader::commit` with zero candidates is a no-op. The server starts
and serves normally. This means the wiring can be merged without
requiring any extension to be deployed alongside it.

### R3 — cross-workspace deps are path deps, not published versions

`examples/notes/Cargo.toml` will reference `starter-ext-host`,
`starter-ext-server`, `starter-ext-mcp`, `starter-ext-grpc` via
`path = "../starter-extensions/crates/<name>"`. This mirrors exactly
what a third-party consumer does once the crates are published to
crates.io. The path form works today without publishing; the switch to
version-pinned deps is a `Cargo.toml` one-liner when the time comes.

### R4 — the gRPC backplane is not the consumer's gRPC service

`starter.ext.grpc.v1.ExtensionGrpc` is the extension substrate's own
service. The consumer's `NoteService` (defined in `proto/notes.proto`)
is registered on the same tonic `Server` as a separate service. The
two share a port and a TLS config but have entirely separate service
definitions. This is tonic's normal multi-service model; no special
glue is needed.

### R5 — UI federation does not bundle the extension's React

The extension's `remoteEntry.js` is served as a static file from the
extension's bundle directory. The host frontend fetches it at runtime
via the Module Federation runtime in `@nube/starter-ext-ui`. React is
a singleton negotiated between host and remote at load time — the
extension does not bundle its own copy. This is enforced by the
`singletons: { react: { version: "..." } }` declaration in
`remoteEntry.ts` (as shown in `starter-extensions/examples/notes/
ui-src/remoteEntry.ts`).

## What is explicitly out of scope

- **Migrating `examples/notes` off its own `NoteSearchTool`** — the
  existing MCP tool registration stays. The extension adapter adds new
  tools from loaded extensions; it does not replace the inline
  registration.
- **Extension persistence across restarts** — `Loader::scan` re-reads
  the bundle directory on every startup. No database table tracks which
  extensions were loaded before. Persistence of extension state (enable/
  disable preferences) is a future `starter-ext-config` concern.
- **Hot-reload** — adding or removing extensions without restarting the
  server is a future `Watcher` concern. The registry is immutable after
  `seal()`; live reload would require a new registry generation and a
  clean handoff of in-flight requests.
- **Extension marketplace / distribution** — how operators install
  extension bundles is out of scope. The loader consumes whatever is in
  the configured directory; how it gets there is the operator's concern.
