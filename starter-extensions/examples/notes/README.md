# notes — cross-surface extension example

One extension bundle that contributes across **every** surface the
`starter-extensions` substrate ships in v0.1: MCP tools, REST routes,
CLI subcommands, and a UI panel — all loaded through the real
`starter-ext-host` loader, dispatched through the real adapter
crates. The counterpart to `hello-builtin` (tools only) /
`hello-cli` (CLI only) / `hello-ui` (UI only), bundled into one
extension to prove the surfaces compose.

If you're trying to answer "how do I write an extension that does
more than one thing?" this is the worked example.

## What this proves

| Surface | Manifest | How it's reached | Where to look |
|---|---|---|---|
| MCP tools | `contributes.tools[]` | `POST /tools/<id>` via `starter-ext-server`'s REST adapter, dispatched into the shared `BuiltinTable` | [src/lib.rs](src/lib.rs), [block.yaml](block.yaml) |
| REST routes | `contributes.rest[]` | `POST /notes`, `GET /notes` — same `BuiltinTable` closure as the tools | [src/lib.rs](src/lib.rs), [block.yaml](block.yaml) |
| CLI subcommands | `contributes.cli[]` | `starter-notes notes-add`, `notes-list` — surfaced via `starter-ext-cli` into the standard `starter-cli::CommandRegistry`, indistinguishable from `health` / `openapi` | [src/lib.rs](src/lib.rs) (`cli_add`, `cli_list`) |
| UI panel | `contributes.ui` | `GET /extensions/com.nube.notes/ui/remoteEntry.js` from the `starter-ext-server` admin slice; mounted into the `sidebar` slot via Module Federation | [ui-src/Panel.tsx](ui-src/Panel.tsx), [ui-src/remoteEntry.ts](ui-src/remoteEntry.ts) |

The loader is the real one:

```
Loader::scan(bundle_root).validate_all() → ExtensionRegistry::seal()
```

No inline registration, no in-memory shortcut, no test fixture. The
binary discovers `block.yaml` on disk, runs the full two-phase commit,
and the resulting `Arc<ExtensionRegistry>` is what every adapter
consumes ([src/main.rs](src/main.rs)).

## Try it

```bash
# From the starter-extensions workspace.
cargo build -p starter-notes-ext
./target/debug/starter-notes --help            # see every surface as subcommands

# Serve REST + admin/UI on 127.0.0.1:8080.
./target/debug/starter-notes serve &

# REST.
curl -s -X POST localhost:8080/notes \
  -H 'content-type: application/json' -d '{"body":"first"}'
curl -s -X POST localhost:8080/notes \
  -H 'content-type: application/json' -d '{"body":"second"}'
curl -s localhost:8080/notes

# MCP tool — auto-mounted at POST /tools/<id> by the REST adapter.
curl -s -X POST localhost:8080/tools/com.nube.notes.search \
  -H 'content-type: application/json' -d '{"query":"second"}'

# Admin slice — proves the loader committed the bundle.
curl -s localhost:8080/extensions
curl -sI localhost:8080/extensions/com.nube.notes/ui/remoteEntry.js

# CLI — separate process, no shared state with the server.
./target/debug/starter-notes notes-add --body "from terminal"
./target/debug/starter-notes notes-list
```

## What's NOT here on purpose

- **No gRPC.** `contributes.grpc` parses in the manifest but there's
  no `starter-ext-grpc` adapter crate yet — the schema is defined,
  zero wiring exists. Adding it would mean building the adapter
  first (substrate work). The original
  [`examples/notes`](../../../examples/notes) keeps gRPC inline in
  its host binary; this example does not because it's strict about
  "everything goes through an extension contribution."
- **No persistence.** `NoteStore` is an in-memory `OnceLock<Mutex<Vec<Note>>>`
  process-global ([src/lib.rs](src/lib.rs)). Restarting `serve` drops
  every note. CLI invocations are separate processes — each gets an
  empty store. A real product would put state behind a future `kv:`
  capability surfaced through `Ctx`; the example punts on that to
  stay focused on contribution shapes.
- **No auth.** `AuthGate` is the manifest field for `require_role` /
  `require_scope` on every contribute entry; the REST adapter's
  `with_role` / `with_scope` wraps the handler when set. The example
  leaves every gate empty. Wiring an `Authenticator` would mean
  building one (token-claim like `examples/notes`, or otherwise) —
  orthogonal to the substrate demonstration.
- **No bundled UI build output.** [ui/remoteEntry.js](ui/remoteEntry.js)
  is a placeholder. A real build wires
  [ui-src/remoteEntry.ts](ui-src/remoteEntry.ts) through rspack/webpack
  with the Module-Federation plugin and emits the runtime bundle to
  that path. The host's admin slice serves whatever's there.

## One implementation note

The proc-macro's `register_static_table!` only dispatches
`contributes.tools[]` — its generated closure rejects unknown ids
with "tool not declared in manifest." Because this extension also
contributes REST routes (which the REST adapter looks up in the
*same* `BuiltinTable` by contribute id), the bundle hand-rolls its
table entry in [`build_builtin_table`](src/lib.rs) so one closure
handles every cross-surface id. The proc-macro's compile-time check
that every declared tool has a matching `handle_*` method is still
in force via the `NotesToolHandlers` impl. The v0.1 proc-macro
covers tools only; future versions could generalise.
