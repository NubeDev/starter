# Rubix extensions — developer guide

A rubix extension is a self-contained bundle directory the agent picks
up at boot. It can contribute MCP tools, flow node-kinds, `SKILL.md`
bundles, and a Module-Federation UI panel. Multiple flavours
(`process`, `builtin`, `wasm`) share one trait surface — pick the
flavour, write the handler once, ship.

The framework itself lives upstream in
[starter-extensions/](../../../../starter-extensions/) — the SDK
(`starter-ext-sdk`), supervisor, host loader, REST/UI adapters, and
PG store. This page covers how rubix consumes that framework and
how to write a new rubix-owned extension.

---

## 1. Bundle layout

A bundle is a directory under `cfg.extensions.dir` (the rubix dev
config defaults to [rubix/extensions/](../../../extensions/)). The
loader scans every immediate child for a `block.yaml`; nothing else
about the directory name matters except that it must be unique.
Convention: name it after the extension id, e.g.
`com.acme.weather/`.

```
com.acme.weather/
├── block.yaml            manifest (REQUIRED)
├── README.md
├── Makefile              build / install / load helpers (recommended)
├── process/
│   ├── Cargo.toml        crate that builds runtime.bin
│   └── src/main.rs
├── <runtime.bin>         the installed binary, sibling to block.yaml
├── kinds/                JSON Schemas + per-contribution markdown
├── skills/<id>/SKILL.md  shipped skill bundles
├── flows/*.yaml          shipped flow definitions
└── ui/
    └── remoteEntry.js    Module-Federation entry
```

The supervisor `chdir`s to the bundle root before exec, so all
paths in `block.yaml` are bundle-relative.

The canonical reference bundle is
[rubix/extensions/com.rubix.example/](../../../extensions/com.rubix.example/).
Copy it to start a new extension.

---

## 2. `block.yaml`

The full schema is the serde struct
[`Manifest`](../../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs)
in `starter-ext-spi`. Every nested struct uses
`#[serde(deny_unknown_fields)]` — a typo is a load-time error, not a
silent ignore.

### 2.1 Required metadata

```yaml
id: com.acme.weather        # reverse-DNS; the bundle's identity
v: 1                        # manifest schema version (always 1 today)
version: 0.1.0              # semver of THIS bundle
display_name: Weather Reader
```

### 2.2 Runtime

Exactly one flavour. The trait body (tool handlers, node behaviours,
…) is identical across flavours; only the entry-point macro changes.

```yaml
runtime:
  kind: process              # process | builtin | wasm
  bin: weather-reader        # for process: bundle-relative binary path
  # crate_name: weather      # for builtin: crate linked at host build
  # wasm_path: weather.wasm  # for wasm:    bundle-relative .wasm
```

Process flavour also accepts an optional `supervision:` block (restart
caps, backoff, health-ping cadence, shutdown grace). Omit it for the
defaults (5 restarts within 60 s, 200 ms→30 s exponential backoff,
5 s health pings, 5 s shutdown grace).

### 2.3 `contributes:`

All fields are optional and additive.

| Field | Adapter | Use when you want to … |
|---|---|---|
| `tools[]` | starter-ext-mcp | expose an MCP tool over `tools/list` + `tools/call` |
| `nodes[]` | starter-ext-flow | add a node kind to the flow palette |
| `skills[]` | starter-ext-flow | ship `SKILL.md` bundles (quarantined) |
| `ui` | starter-ext-server + frontend | mount a React panel via Module Federation |
| `cli[]` | starter-ext-cli | add a CLI subcommand |
| `rest[]` | starter-ext-server | add a REST route (JSON / SSE / NDJSON) |
| `grpc[]` | starter-ext-grpc | add a gRPC RPC |
| `workers[]` | starter-ext-workers | run a periodic background job |
| `i18n` | starter-ext-server | serve per-locale catalog fragments |

Rubix wires `tools`, `nodes`, `skills`, and `ui` today; the other
adapters compile but the rubix boot composer does not mount them yet.

### 2.4 Tools

Each `tools[]` entry declares a tool. The handler lives in your
crate; the SDK macro mangles the id into a method name.

```yaml
contributes:
  tools:
    - id: com.acme.weather.current   # MUST be id or dotted descendant
      input_schema:  kinds/current_in.json   # JSON Schema 2020-12
      output_schema: kinds/current_out.json
      description_file: kinds/current.md     # static markdown, never templated
      auth:                                  # optional
        require_role: user                   # default: inherit adapter's
        require_scope: weather:read
```

The id `com.acme.weather.current` produces the trait method
`handle_com_acme_weather_current(&self, ctx, params) -> Result<Value>`.

### 2.5 Flow node-kinds

```yaml
contributes:
  nodes:
    - kind: com.acme.weather.fetch    # must descend from id
      settings_schema: kinds/fetch_settings.json
      description_file: kinds/fetch.md       # optional
      facets: [transport, transform]         # editor palette tags
      streaming: false                        # advisory
```

Today slice-A binds a placeholder behaviour that returns
`NodeError::Domain { code: "no_behaviour_bound" }`. Slice-B will route
`flow.node.invoke` to the same child process that serves the tools.

### 2.6 Skills

```yaml
contributes:
  skills:
    - dir: skills           # bundle-relative directory of <id>/SKILL.md
```

The host folds discovered bundles into `SkillRegistry::extend(...)`
**quarantined regardless of the bundle's `trust:` frontmatter** — an
extension cannot self-approve a skill. An operator approves by
content hash; one byte changes → re-quarantined.

### 2.7 UI

```yaml
contributes:
  ui:
    entry: ui/remoteEntry.js   # bundle-relative path
    exposes:
      - name: Main             # component name; matches handle.register key
        module: "./Main"       # MF v1 module path string (cosmetic for us)
        slot: main             # frontend slot id
```

The server resolves `entry`'s parent directory as the UI root, then
serves `/api/v1/extensions/{id}/ui/{*path}` from it. So an entry of
`ui/remoteEntry.js` is fetched as
`/api/v1/extensions/{id}/ui/remoteEntry.js` (the leading `ui/` is
stripped, otherwise you get `/ui/ui/remoteEntry.js` → 404).

The rubix-frontend `/extensions` route has one mounted slot today:
`<ExtensionSlot id="main">`. Use `slot: main` to render there.

---

## 3. Writing the process binary

Build inside the
[rubix/extensions/](../../../extensions/Cargo.toml) sibling
cargo workspace. Per SCOPE R8 your crate may depend ONLY on
`starter-ext-sdk` — never on `rubix-domain`, `rubix-tools`,
`rubix-agent`, or any `starter/crates/*`. The workspace boundary
enforces this.

`process/Cargo.toml`:

```toml
[package]
name    = "weather-reader"
edition = "2021"

[[bin]]
name = "weather-reader"
path = "src/main.rs"

[dependencies]
starter-ext-sdk = { path = "../../../../starter-extensions/crates/starter-ext-sdk",
                    default-features = false, features = ["process"] }
tokio = { version = "1", default-features = false,
          features = ["rt", "macros", "io-std", "io-util", "time"] }
```

The four `..` segments hop out to the repo root, then into the
upstream workspace. The `process` feature picks the JSON-RPC-over-
stdio transport.

`process/src/main.rs`:

```rust
use starter_ext_sdk::Extension;

#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Weather;

starter_ext_sdk::requires! {
    name = WeatherCtx,
    capabilities = [],
}

impl WeatherToolHandlers for Weather {
    type Ctx = WeatherCtx;

    fn handle_com_acme_weather_current(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        // your impl here
        Ok(params)
    }
}

starter_ext_sdk::register_process_main! {
    extension: Weather,
    ctx: WeatherCtx,
    instance: Weather,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("weather-reader exiting: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
```

Notes:
- `#[derive(Extension)]` parses `block.yaml` at compile time and
  emits the `<Name>ToolHandlers` trait with one method per
  `contributes.tools[]` entry. Every method is required; omit one and
  the build fails with `not all trait items implemented`.
- `requires! {}` declares the host capabilities your extension needs
  (e.g. `Capability::http`, `Capability::secrets`); the host injects
  matching handles into the generated `Ctx` struct.
- `register_process_main!` emits `pub async fn run() ->
  starter_ext_sdk::Result<()>` driving the JSON-RPC loop.

Register your crate in
[rubix/extensions/Cargo.toml](../../../extensions/Cargo.toml)
`workspace.members`.

---

## 4. UI panel (`ui/remoteEntry.js`)

A minimal hand-authored federation entry: no build step, plain ESM,
default export with `singletons` (negotiation map) and `init(handle)`
(registration callback). React is provided by the host — never bundle
your own.

```js
const factory = {
  singletons: {
    react: { version: "19.0.0" },
  },
  init(handle) {
    const React = handle.singletons.react;
    const h = React.createElement;

    function Main(props) {
      return h("div", { "data-ext-slot": props.slotId },
        "hello from com.acme.weather");
    }

    handle.register({ components: { Main } });
  },
};

export default factory;
```

The component name(s) registered here MUST match the `name:` values
in `contributes.ui.exposes`. The host's `<ExtensionSlot id="…">`
mounts every registered component whose `slot:` matches.

A larger, fetch-driven example lives in
[rubix/extensions/com.rubix.example/ui/remoteEntry.js](../../../extensions/com.rubix.example/ui/remoteEntry.js).

---

## 5. REST surface

The rubix agent merges `starter_ext_server::router(admin)` under
`/api/v1/extensions/*`.

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `GET` | `/api/v1/extensions` | `Role::Admin` | List records (id, version, state, enabled, contributes summary) |
| `GET` | `/api/v1/extensions/{id}` | `Role::Admin` | Full record incl. parsed manifest |
| `GET` | `/api/v1/extensions/{id}/events` | `Role::Admin` | SSE — lifecycle / log / error events |
| `POST` | `/api/v1/extensions/{id}/enable` | `Role::Admin` | Persist `Enabled` in PG, ensure supervisor up |
| `POST` | `/api/v1/extensions/{id}/disable` | `Role::Admin` | Persist `Disabled` in PG, drop supervisor |
| `POST` | `/api/v1/extensions/install` | `Role::Admin` | Multipart tarball → unpack + validate + reload |
| `DELETE` | `/api/v1/extensions/{id}` | `Role::Admin` | Stop, mark `Disabled`, remove directory |
| `GET` | `/api/v1/extensions/{id}/ui/{*path}` | unauthed | Federation bundle assets (ETag-cached) |
| `GET` | `/api/v1/extensions/{id}/i18n/{lang}` | unauthed | Per-locale catalog fragment |

There are no `start` / `stop` / `restart` endpoints — lifecycle is
driven by `enable` / `disable`. The supervisor's restart loop handles
crash recovery automatically.

UI and i18n routes are deliberately unauthenticated so the frontend
host can fetch public assets without a CSRF dance.

All lifecycle writes audit a `changelog` row (`kind =
extension.<action>`) through the existing `changelog_middleware`.

---

## 6. Boot sequence

`rubix-agent`'s [`boot::extensions::build_extension_admin`](../../../crates/rubix-agent/src/boot/extensions.rs)
composes the upstream primitives in a single async fn:

1. Apply `PgEnablementStore`'s own migration against the shared rubix
   PG pool.
2. Canonicalise `cfg.extensions.dir` to an absolute path. The
   supervisor exec's `bundle_dir.join(runtime.bin)`; a relative
   `bundle_dir` would double-resolve and ENOENT.
3. `Loader::scan(dir).validate_all()` — two-phase manifest load.
   A missing dir is treated as an empty load (cargo-run from a fresh
   checkout still boots).
4. `Loader::commit(records, &mut registry)` then `registry.seal()`.
5. Read persisted state via `PgEnablementStore::list_all()`; rows
   missing from PG default to `Enabled` when
   `cfg.extensions.autostart_enabled_records = true` (the dev
   default), else `Disabled`.
6. For each `Enabled` process-flavour record,
   `DefaultSupervisorFactory::spawn(record)` runs the binary under
   the restart-with-backoff policy.
7. Materialise an `ExtensionAdmin` handle and merge the upstream
   router into the agent's main `axum::Router`.

Boot-time autostart audit rows are written under the synthetic
principal `system:extensions-autostart` so operators can tell a boot
replay apart from an operator-issued enable.

Grep `INFO rubix.boot.extensions loaded=N autostarted=M` to confirm.

---

## 7. PG persistence

Enable/disable state survives restart because `PgEnablementStore`
upserts into
`extensions_enablement(extension_id, state, updated_at, updated_by)`.
The `updated_by` column threads through from `set_as(actor_id, …)` so
audits stay honest even when a restart replays the table.

The migration ships under
`starter-ext-store-pg/src/migrations/0001_extensions_enablement.sql`
and is applied by the boot composer — rubix's own `migrations/` tree
does not number it.

---

## 8. Build + load workflow

The example bundle ships a [Makefile](../../../extensions/com.rubix.example/Makefile)
with the canonical targets. From a bundle directory:

```bash
make build      # cargo build --release -p <crate-name>
                # (cargo target dir is forced OUT of rubix/extensions/
                #  so the loader doesn't see it as a phantom bundle)
make install    # install -m 0755 <binary> ./
make load       # delegates to `make -C rubix restart` (one-shot
                # process restart — there is no live /reload route)
make test       # auth + GET /api/v1/extensions/<id>; jq-summary the
                # state, enabled, and contributes lists
make status     # compact one-line state probe
make logs       # tail -F /tmp/rubix-agent.log
make all        # build + install + load + test
```

After `make load`, expect to see:

```
state:       running        # process flavour, after init handshake
enabled:     enabled        # PG-persisted
restart_count: 0            # > 0 = supervisor is crash-looping
```

If `state` stays at `validated` and `restart_count` climbs, check
`/tmp/rubix-agent.log` for the
`starter_ext_supervisor::supervisor: spawn / init failed` line — the
`err=` field names the failure (binary missing, manifest hash
mismatch, handshake timeout, …).

---

## 9. Frontend wiring

The rubix-frontend [`/extensions`](../../../frontend/src/routes/extensions.tsx)
route lists every loaded extension, surfaces per-row Enable/Disable
buttons, and offers a **Load UI** action for any extension that
contributes a `ui` block.

Clicking Load UI dynamic-imports the federation entry, calls
`factory.init(handle)` with the host's negotiated singletons, and
registers the result with the shared
[`ExtensionHostProvider`](../../../../starter-extensions/packages/starter-ext-ui/).
`<ExtensionSlot id="main">` re-renders to mount the newly registered
panel.

A panel can call any agent route using plain
`fetch('/api/v1/...', { credentials: 'same-origin' })` — cookies are
inherited from the operator session.

---

## See also

- [Upstream framework scope (authoritative)](../../../../starter-extensions/DOCS/extensions/scope/SCOPE.md)
- [Manifest schema source of truth](../../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs)
- [Reference bundle](../../../extensions/com.rubix.example/)
- [hello-process upstream example](../../../../starter-extensions/examples/hello-process/)
- [hello-ui upstream example](../../../../starter-extensions/examples/hello-ui/)
