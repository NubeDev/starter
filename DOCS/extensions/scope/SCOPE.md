# starter-extensions — Scope

## One-line summary

`starter-extensions` is a small set of focused libraries — Rust crates +
React/TS packages — that a starter-based product imports to load,
supervise, and surface third-party **extensions**: code units that
contribute tools, routes, UI panels, or background workers without
forking the host. One trait, three packaging flavours (in-process,
WASM, child process), one YAML manifest, one supervisor, one UI
federation runtime.

It is **not a framework that consumers build on top of** and **not a
plugin marketplace**. A consumer's product imports the pieces it needs
(`starter-ext-host`, optionally `starter-ext-supervisor` and
`starter-ext-wasm`, the UI runtime), wires them into its
`ServerBuilder`, and ships. An extension author imports
`starter-ext-sdk`, writes one trait impl + a `block.yaml`, and the
host loads it. The boundary between host and extension is the same
shape as the boundary between an OS kernel and a userspace process —
not the boundary between a framework and an app.

## Why this exists

Across several Rust projects (codeless, rubix-agent, future products)
the same extension substrate keeps being re-implemented:

- A trait that an extension author implements once and compiles into
  three flavours (linked-in, WASM-sandboxed, child-process-supervised).
- A YAML manifest declaring id, version, runtime, capability
  requirements, and UI/tool contributions.
- A two-phase loader that validates every extension before committing
  any to the registry.
- A supervisor that spawns child-process extensions over stdio
  JSON-RPC, restarts them on crash with intensity caps and backoff,
  forwards stderr, and exposes their state over an HTTP endpoint.
- A WASI-p2 host with default-deny capability grants.
- A Module-Federation runtime that mounts extension-provided React
  panels into named host slots.

Codeless built it for AI-tool plugins. Rubix-agent built it for
graph-node blocks. The two projects converge on the same ~8 ideas;
each rolled their own and the two have drifted. `starter-extensions`
extracts the kernel — the parts that are *not* domain-specific to
agents, flow graphs, or AI tools — into reusable crates.

This is the **third implementation**, which is the right time to
extract: the pattern has been validated twice in production, the
shared kernel is visible, and the differences (codeless's persona
visibility, rubix's graph-node state) are clearly domain-specific and
do not belong in the kernel.

## Relationship to starter

`starter-extensions` is a **sibling workspace** to `starter`, not a
set of crates inside it.

```
parent-dir/
  starter/                    <- the libraries SCOPE.md describes
  starter-extensions/         <- this scope; depends on starter-spi only
```

This split is load-bearing for three reasons:

1. **SCOPE.md R0** (implicit in "small libraries, not a framework"):
   an extension substrate with a supervisor, WASM host, and federation
   runtime is unambiguously a framework. Keeping it out of `starter`
   preserves the "small libraries you cargo-add" character.
2. **Opt-in by repository.** A consumer who doesn't want extensions
   doesn't depend on this workspace. A consumer who does adds
   `starter-ext-host = "0.1"` to their `Cargo.toml` and is done.
3. **Separate release cadence.** The extension framework will iterate
   faster (and break) than `starter`'s small libraries should.

The two workspaces share **only `starter-spi`**. Extension contributions
that return tools use `starter_spi::tool::Tool`; capabilities that grant
secret access go through `starter_spi::secrets::SecretStore`. No new
contract crate; `starter-spi` is the seam.

## Hard rules (load-bearing)

These rules are why an extension written today still loads in two
years and why the kernel doesn't accrete domain knowledge. Break one
and the substrate collapses into "yet another plugin framework".

### R1 — One trait, three flavours, one source

An extension author writes a single `ExtensionBehavior` impl on a
single struct. The same source compiles to **builtin** (statically
linked into the host), **wasm** (WASI-p2 component), or **process**
(spawned over stdio JSON-RPC) via mutually-exclusive cargo features.

The trait method signatures, the manifest format, and the host's
dispatch path are identical under all three flavours. The `Ctx` type
the extension receives is also identical *in shape* per extension —
its method set is determined by `requires!{}` at the extension's
compile time, not by the flavour. The same extension built for
builtin, wasm, and process has the same `Ctx` API; two different
extensions built for the same flavour can have different `Ctx` APIs.
Only the entry-point glue (`register_static_table!` /
`wit_bindgen::export!` / a `tokio::main` JSON-RPC loop) differs
between flavours. A linker error is the signal that more than one
flavour is enabled at once. **No flavour-specific traits**; if a
thing is only possible in one flavour, the trait does not expose it.

### R2 — `starter-ext-spi` is the contracts crate; depends only on `starter-spi`

Wire types and trait seams live in `starter-ext-spi`: the
`ExtensionBehavior` trait, the `Manifest` struct, `ExtensionId`,
`Capability`, the supervisor's lifecycle state enum, the JSON-RPC
envelope types, and the error type. Zero runtime logic, zero I/O,
zero process spawning, zero HTTP. **`starter-ext-spi` depends only on
`starter-spi`** and every other crate in this workspace depends on
`starter-ext-spi`.

### R3 — The manifest is the only source of truth for what the extension provides

`block.yaml` is parsed with `deny_unknown_fields`. Names, versions,
descriptions, schemas, prompt templates, capability grants, **the
list of tools and UI contributions** — all declared there.
Descriptions and schemas are **paths to static files** in the
extension bundle, never templated at runtime. Operators read one file
to know what an extension does and what it can touch; the host reads
the same file at load time.

Dispatch is manifest-driven. The host calls the extension by tool id
from the manifest; there is no separate runtime "registration phase"
where the extension informs the host of what it provides. The
`#[derive(Extension)]` proc-macro reads the bundle's `block.yaml` at
the extension's compile time and emits the dispatch table; an impl
missing a handler for a manifest-declared tool (or providing a
handler for an undeclared one) is a compile error in the *extension*,
not a runtime error in the *host*. One source of truth, checked once,
at the extension's build.

For process flavour, the child binary and the host load the same
`block.yaml`. The init handshake includes the manifest's content
hash; mismatch (child built against a different manifest than the
bundle ships) is refused by the supervisor with a clear deploy-time
error.

### R4 — Reverse-DNS ids; namespace ownership enforced

An extension's id is a reverse-DNS string (`com.acme.weather`). Every
identifier the extension contributes — tool ids, panel ids, kind ids —
**must** be the extension id or a dotted descendant
(`com.acme.weather.current`, `com.acme.weather.panel`). The loader
rejects an extension that contributes `sys.core.foo` or
`com.other-vendor.thing`. Reserved prefixes (`sys.*`, `starter.*`)
belong to the host and cannot be claimed.

This rule kills the entire class of "extension A breaks because
extension B shadowed its id" bugs.

### R5 — Stateless behaviours; state goes through `Ctx`

`ExtensionBehavior` methods take `&self`, never `&mut self`. The
extension struct holds no fields except unit. Per-instance state lives
in:

- The host-provided store (secrets via `SecretStore`, configuration
  via the manifest's `Config` schema, persistent data via a future
  `kv` capability).
- The supervisor's per-extension state machine (lifecycle, restart
  count, last error) — read-only to the extension.

The discipline is what keeps the three flavours interchangeable. WASM
plugins get fresh instances per call; in-process plugins live for the
host's lifetime; process plugins survive restarts. None of those
differences leak through the trait if state is not in the struct.

### R6 — Capabilities are declared, granted, and (where the runtime permits) enforced

Two distinct concepts share the manifest:

- **`requires:`** — host *interface* dependencies. An extension states
  "I need `starter.spi.tool` at major 1." The host validates that every
  required interface is provided at a compatible version; mismatch is a
  hard load error.
- **`capabilities:`** — runtime *grants* the operator scopes. An
  extension that needs HTTP out lists `http_out` in `requires`; the
  operator's `capabilities.http_out: ["api.weather.gov"]` is the
  authority allowlist that scopes that grant. **Setting
  `http_out: []` is legal**: the extension loads but every outbound
  HTTP call is denied at runtime (the operator has chosen to
  neutralise the grant without disabling the extension). **Omitting
  `http_out` entirely when the extension required it is a load error.**

The `Ctx` handle the SDK gives the extension only exposes methods for
declared capability categories. The `requires!{}` macro expands into
a generated `Ctx` newtype whose method set is determined by the
macro's arguments. An extension that did not declare `http_out` has
no `http()` method available — the call does not type-check. This is
identical across flavours (R1): two different extensions on the same
host can have differently-shaped `Ctx` types; the same extension built
for all three flavours has the same `Ctx` type.

Enforcement at runtime varies by flavour:

- **WASM**: hard-enforced by the WASI host. A component without
  `http_out` granted has no `wasi:http` import linked.
- **Process**: enforced at the JSON-RPC wire boundary. The host
  rejects child requests for methods backing un-declared capabilities
  and increments a `capability_violation` counter visible on
  `GET /extensions/<id>`.
- **Builtin**: not enforced. Builtin extensions run in the host's
  address space and are trust-equivalent to host code. Capability
  declarations exist for documentation and consistency, not isolation.
  An operator who wants enforced isolation packages the extension as
  WASM or process.

### R7 — Static metadata, never runtime-templated

Tool descriptions, JSON Schemas, prompt templates, panel labels — all
live as files in the extension bundle and are read by the host at load
time. **Never** assembled at runtime from extension-supplied
fragments. This is enforceable, reviewable, and (critically for the
AI-tool case codeless cares about) anti-prompt-injection.

The host caches loaded artefacts; an extension cannot mutate them
between calls.

### R8 — Default-deny for WASM; advisory-deny for process; explicit-grant for both

The WASM host (`starter-ext-wasm`) starts every component with **no
WASI capabilities**. The manifest's `capabilities:` block lists
explicit grants. Capability *categories* are typed (`http_out:
["api.example.com"]`, not `fs: "/some/path"`). Per-call fuel, memory,
and wall-clock caps are configured host-side, not in the manifest.

The process supervisor (`starter-ext-supervisor`) cannot enforce
capability use inside the child. It enforces at the **JSON-RPC wire
boundary**: if a child calls a host method it did not declare, the
host returns an error and logs a `capability_violation` event.
Resource caps (cgroups/rlimits) are deferred to v0.2 — they are not
load-bearing for v0.1's trust model.

### R9 — One supervisor, one restart policy, no supervision groups in v0.1

Each process-flavour extension is its own supervisor subtree.
Restart policy is per-extension (`always | on_crash | never` —
restart on any exit, restart only on abnormal exit, never restart)
with an intensity cap (max N restarts in M seconds) and exponential
backoff with jitter. **No supervisor groups** (one-for-all,
rest-for-one) in v0.1 — every extension restarts independently.

The reason is empirical: groups are valuable when extensions are
tightly coupled (rare in this model — extensions are isolated by
design), and they are a source of "why did extension B die when
extension A crashed?" bugs. Adding groups later is additive in the
manifest (`supervision.group: storage-tier`); not adding them now
keeps the supervisor's state machine an order of magnitude simpler.

### R10 — One IPC wire format: stdio JSON-RPC

Process-flavour extensions communicate with the host over **stdin /
stdout**, framed as JSON-RPC 2.0 messages (Content-Length headers,
LSP/MCP-style). No Unix-domain sockets, no gRPC, no shared memory.

Stdio is debuggable (`cat trace.jsonl | weather-driver`),
language-agnostic (any runtime that can read stdin), and free of
socket-lifecycle bugs. Latency is irrelevant at expected message
rates (an extension is not a real-time control loop). When a consumer
shows up with a real performance need, the supervisor adds a
side-channel — it does not replace the primary channel.

stderr is forwarded to the host's tracing subsystem with the
extension's id as a span tag.

### R11 — UI extensions via Module Federation; shared singletons negotiated by host

Frontend extensions ship a `remoteEntry.js` (Module Federation
bundle). The host (`starter-ext-ui`) loads remotes at runtime,
mounts exposed modules at named slots (`<ExtensionSlot id="sidebar"/>`),
and negotiates shared singletons (React, react-dom, the query lib,
the store) so two extensions on the same page do not load duplicate
copies.

UI extensions **cannot** issue raw `fetch` calls. They consume the
host RPC client via `useHostClient()` from `@starter/ext-sdk-ts`,
which routes through `starter-client-ts` so auth, tracing, and
retry are uniform.

`starter-ext-ui` is **separate from `starter-ui-kit`**. A consumer
who renders shadcn primitives without extensions does not pay for
the Module Federation runtime.

**Named singletons (current set).** The host singleton registry
contains, at minimum:

| Singleton id | Source | Purpose |
|---|---|---|
| `react` | host runtime | one React per page |
| `react-dom` | host runtime | matched to React |
| `@nube/starter-ui-core/preferences` | `@nube/starter-ui-core` | host's resolved `PreferencesContext` (R9 of the prefs SCOPE) |
| `@nube/starter-ui-core/i18n` | `@nube/starter-ui-core` | host's `react-intl` `IntlShape` (R9 of the prefs SCOPE) |

Singleton ids match the **package + subpath** an extension would
`import` if linked directly. Each is independently versioned by
major; mismatch on any one fails registration for that extension
without affecting others. Minor drift loads with a
`extension.singleton_minor_drift` telemetry event; patch drift is
silent. The SDK package (`@nube/starter-ext-sdk-ts`) versions in
lockstep with the prefs/i18n singleton major.

Extension authors read these singletons via the SDK hooks
(`useHostPrefs`, `useHostTranslate`, `useHostFormatters`) rather
than reaching into the singleton table directly. The SDK auto-
prefixes message keys with the calling extension's id so
`com.nube.hello/i18n/en.json` `greeting` resolves as
`com.nube.hello.greeting` in the merged bundle.

### R12 — Comments explain *why*, never *what*. No session-progress chatter

Same as starter R8. Doc-comments on every public item; no
`// FIXED:` banners, no emoji, no progress logs. TODOs carry a name
or ticket.

### R13 — Extensions contribute to every transport the host exposes through one seam

An extension is not "an MCP tool" or "a REST handler" or "a CLI
command" — it is a contributor that can produce **any of those** from
the same trait + manifest. The host's *transport adapters* turn
manifest contributions into transport-native objects:

| Contribution        | Adapter crate                | Adapter output                            |
| ------------------- | ---------------------------- | ----------------------------------------- |
| `contributes.tools` | `starter-ext-mcp` (optional) | registers `Tool` impl in `ToolRegistry`   |
| `contributes.tools` | `starter-ext-server` (REST)  | surfaces same `Tool` at `/tools/<id>`     |
| `contributes.cli`   | `starter-ext-cli` (optional) | registers `Command` in `CommandRegistry`  |
| `contributes.rest`  | `starter-ext-server`         | merges `axum::Router` fragment            |
| `contributes.grpc`  | `starter-ext-grpc` (slot)    | registers tonic `Service` impl            |
| `contributes.workers` | `starter-ext-workers`      | schedules tick handler on host scheduler  |
| `contributes.warehouse` | `starter-ext-warehouse` (planned) | registers cleaners / rules / marts into the warehouse catalog (see note below) |
| `contributes.ui`    | `starter-ext-ui`             | mounts MF remote at named slot            |

**One contribution can surface in multiple transports.** A
`contributes.tools` entry is reachable from MCP (via `starter-mcp`)
and from REST (via `starter-server`'s `/tools/<id>`) without the
extension author writing two handlers. The adapter decides whether to
expose the contribution; the extension declares what exists.

For builtin flavour, the adapter calls into the extension's
statically-linked dispatch table directly. For process and wasm
flavours, the adapter forwards the call over the existing JSON-RPC
channel — every transport reuses the **one** stdio channel from R10;
adapters do not open new wire formats to talk to extensions.

Adding a new transport to the host (websockets, GraphQL, anything
future) is one new `starter-ext-<transport>` adapter crate, one new
`contributes.<transport>` block in the manifest schema. **The trait
does not change.** Extensions written today keep working when a new
transport adapter ships, as long as they do not opt in. Operators
turn adapters on with cargo features.

This is the seam that justifies the whole framework. Without it,
"extension" would mean a different thing per surface and the host
would accrete bespoke plugin hooks per transport — exactly the drift
the parent SCOPE.md is trying to prevent.

#### Note — `contributes.warehouse` (planned, focus area)

The near-term focus is integrating this framework with the
[TimescaleDB warehouse](../../Warehouse/SCOPE.md) (see
[ADR-004](../../storage/ADR-004-timescaledb-warehouse.md)). A
`contributes.warehouse` block in `block.yaml` will let extensions
ship named **cleaners** (transforms between L1 `raw_events` and
L2 `samples` / `events`), **rules** (parameterised SQL evaluated on
demand, Insights-shaped), and **marts** (`MartSpec` catalog rows
that generate **continuous aggregates** on TimescaleDB
hypertables). Each entry points to
a static SQL or TOML file in the bundle (per R7 — never templated
at runtime); the `starter-ext-warehouse` adapter reads, hashes, and
validates them at load, then registers them in the Postgres catalog
with `created_by = 'ext:<extension-id>'`.

Reverse-DNS ids (R4) become the catalog / continuous-aggregate names
— namespace collisions between extensions are structurally impossible
(`com.acme.energy.cleaner.normalise_kwh` is the cleaner's name).
A new `warehouse_write` capability (R6) scopes which tables an
extension may register cleaners against, listed per-table the same
way `http_out` is listed per-host.

**Lifecycle ownership.** Extension-authored marts and cleaners follow
the lifecycle defined in
[Warehouse W12](../../Warehouse/SCOPE.md#w12--mart-lifecycle-is-governed)
— the `ext:` author type starts `pending` and auto-promotes to `live`
on successful DDL apply (the manifest hash + operator enable/disable
is the trust seam). **The `starter-ext-warehouse` adapter does not
write the `status` column directly.** It calls `mart.promote` (or
`cleaner.promote`) through the warehouse capability API after DDL
succeeds. The warehouse owns the status state machine; the extension
adapter is a caller. This ensures the three-author lifecycle table in
W12 is the single source of truth for all status transitions.

A separate SCOPE for `starter-ext-warehouse` will land alongside
the adapter crate. Nothing in the trait or manifest core changes;
this is purely an additive transport adapter per R13.

## Repo layout

```
starter-extensions/                                  <- this workspace
  Cargo.toml                                         <- workspace
  pnpm-workspace.yaml

  crates/
    starter-ext-spi/                                 <- R2: contracts.
                                                        Depends only on starter-spi.
                                                        ExtensionBehavior trait,
                                                        Manifest, ExtensionId,
                                                        Capability, LifecycleState,
                                                        JsonRpc envelope, Error.

    starter-ext-sdk/                                 <- What extension authors import.
                                                        #[derive(Extension)] proc-macro,
                                                        requires!{} declaration macro,
                                                        Ctx handle, flavour-specific
                                                        entry-point glue (mutually-
                                                        exclusive cargo features:
                                                        `builtin` | `wasm` | `process`).

    starter-ext-host/                                <- Manifest loader (YAML),
                                                        two-phase validator,
                                                        namespace ownership check,
                                                        capability compatibility check,
                                                        ExtensionRegistry. No spawning,
                                                        no WASM, no I/O beyond reading
                                                        bundle files at load time.

    starter-ext-supervisor/                          <- Process-flavour supervisor.
                                                        Spawns children via
                                                        tokio::process, stdio JSON-RPC
                                                        framing, restart strategy
                                                        (always/on_crash/never)
                                                        with intensity cap and backoff,
                                                        health checks, stderr forwarding,
                                                        event ring buffer per extension.

    starter-ext-wasm/                                <- OPTIONAL, feature-gated.
                                                        WASI-p2 component host on
                                                        wasmtime. Default-deny capability
                                                        model; per-call fuel + memory +
                                                        deadline caps.

    starter-ext-server/                              <- HTTP transport adapter + admin
                                                        routes. Surfaces
                                                        contributes.rest as merged Router
                                                        fragments; surfaces
                                                        contributes.tools at /tools/<id>;
                                                        owns admin endpoints:
                                                          GET  /extensions
                                                          GET  /extensions/<id>
                                                          GET  /extensions/<id>/events
                                                          POST /extensions/<id>/enable
                                                          POST /extensions/<id>/disable
                                                          GET  /extensions/<id>/ui/*
                                                        Depends on starter-server.

    starter-ext-mcp/                                 <- OPTIONAL, feature-gated.
                                                        MCP transport adapter. Registers
                                                        contributes.tools entries into
                                                        starter-mcp::ToolRegistry. Depends
                                                        on starter-mcp + starter-ext-host.

    starter-ext-cli/                                 <- OPTIONAL, feature-gated.
                                                        CLI transport adapter. Registers
                                                        contributes.cli entries as
                                                        Command impls in starter-cli::
                                                        CommandRegistry. Depends on
                                                        starter-cli + starter-ext-host.

    starter-ext-workers/                             <- OPTIONAL, feature-gated.
                                                        Tick-based scheduler that invokes
                                                        contributes.workers handlers at
                                                        manifest-declared intervals.
                                                        Not a job queue — just periodic
                                                        invocation with backoff on error.

    starter-ext-grpc/                                <- OPTIONAL, feature-gated.
                                                        gRPC transport adapter on top of
                                                        starter-grpc + tonic. Surfaces
                                                        contributes.grpc as one
                                                        backplane service
                                                        `starter.ext.grpc.v1.ExtensionGrpc`
                                                        with ListMethods + Invoke (unary)
                                                        + InvokeStream (server-streaming),
                                                        routed by (service, method)
                                                        pairs from the manifest.
                                                        Typed per-extension tonic
                                                        services land additively
                                                        under `starter.ext.grpc.v2`.

  packages/
    starter-ext-ui/                                  <- Host-side Module Federation
                                                        runtime: <ExtensionSlot/>,
                                                        useExtensionHost(),
                                                        registerExtensionRemote().
                                                        Negotiates shared singletons
                                                        (React, query lib, store).
                                                        Zero rubix/codeless coupling.

    starter-ext-sdk-ts/                              <- What UI-extension authors
                                                        import: SlotContribution type,
                                                        useHostClient(), BlockShell,
                                                        slot-context hooks. Forked from
                                                        rubix-workspace/extension-ui-sdk
                                                        with rubix-specific (graph
                                                        node/slot) hooks stripped.

  examples/
    hello-builtin/                                   <- minimal in-process extension
    hello-process/                                   <- same source, process flavour
    hello-wasm/                                      <- same source, WASM flavour
    hello-ui/                                        <- React panel contributing to
                                                        a named slot
```

## Dependency arrow (Rust)

```
starter-spi  (from the starter workspace)
   ↑
   │
starter-ext-spi
   ↑          (everything in this workspace depends on ext-spi;
   │           ext-spi depends only on starter-spi)
   │
   ├── starter-ext-sdk
   ├── starter-ext-host
   ├── starter-ext-supervisor   ──→ starter-ext-host (registry types)
   ├── starter-ext-wasm         ──→ starter-ext-host (registry types)
   │
   │  --- transport adapters (each optional, each feature-gated) ---
   │
   ├── starter-ext-server       ──→ starter-ext-host + starter-ext-supervisor
   │                                + optionally starter-ext-wasm
   │                                + starter-server (parent workspace)
   │                                surfaces: contributes.rest, contributes.tools,
   │                                          contributes.ui (file serving), admin
   ├── starter-ext-mcp          ──→ starter-ext-host + starter-mcp
   │                                surfaces: contributes.tools (MCP transport)
   ├── starter-ext-cli          ──→ starter-ext-host + starter-cli
   │                                surfaces: contributes.cli
   ├── starter-ext-workers      ──→ starter-ext-host (no transport dep)
   │                                surfaces: contributes.workers
   └── starter-ext-grpc         ──→ starter-ext-host + starter-ext-supervisor
                                    + starter-grpc (parent workspace)
                                    surfaces: contributes.grpc
```

**Never** the other way: no crate in `starter-extensions` is consumed
by `starter` itself. The consumer's binary depends on both workspaces;
neither workspace depends on the other in reverse.

## Dependency arrow (TypeScript)

```
starter-client-ts          (from the starter workspace)
        ↑
        │
starter-ext-sdk-ts         (extension authors import this)
        ↑
        │
starter-ext-ui             (host shell imports this to mount extensions)
        ↑
        │
   consumer app shell
```

`starter-ext-ui` consumes `starter-ext-sdk-ts` for the registration
contract types; it does **not** depend on `starter-ui-kit` (design
primitives are separate). An extension author depends on
`starter-ext-sdk-ts` + `starter-ui-kit` (for primitives) +
`starter-client-ts` types — never on `starter-ui-core` directly (that
is the consumer's brain).

## What each crate / package owns

### `starter-ext-spi` (Rust)

- `trait ExtensionBehavior` — flavour-agnostic. Methods:
  `on_init(&self, ctx: &Ctx, cfg: &Self::Config) -> Result<()>`,
  `on_shutdown(&self, ctx: &Ctx) -> Result<()>`, plus an associated
  `Config: serde::Deserialize` type. Tool/RPC dispatch is generated
  from the manifest's `contributes` block; the trait itself does not
  enumerate handlers.
- `struct Manifest` — the deserialised `block.yaml`. Versioned (`v: 1`
  field) so new fields are additive forever.
- `ExtensionId` — newtype around a reverse-DNS string; validates on
  construction.
- `enum Capability` — typed grants (`Secrets(Vec<String>)`,
  `HttpOut(Vec<Authority>)`, `Fs(Vec<PathSpec>)`, `WallClock`,
  `Custom(String)`).
- `enum LifecycleState` — `Discovered | Validated | Starting | Running
  | Stopping | Stopped | Crashed | Failed`. Same enum used by both the
  host registry and the process supervisor.
- `struct JsonRpcEnvelope` — host↔extension wire shape (request/response/
  notification, Content-Length framed).
- `enum Error` — `Manifest(...)`, `Validation(...)`, `Spawn(...)`,
  `Transport(...)`, `Capability(...)`, `ExtensionInternal(...)`.

**Notably absent from `ext-spi`:** no per-flavour types. WASM-specific
WASI bindings live in `starter-ext-wasm`; supervisor-specific restart
state lives in `starter-ext-supervisor`. `ext-spi` is the lowest
common denominator.

### `starter-ext-sdk` (Rust)

What an extension author depends on. Three mutually-exclusive cargo
features (`builtin`, `wasm`, `process`) — exactly one must be enabled
or compilation fails at link time. The trait the author implements is
the **same** under all three; only the generated entry-point glue
differs.

- `#[derive(Extension)]` — generates `id()`, `version()`,
  `manifest_static()` (a reference to the bundle's parsed manifest),
  and the flavour-appropriate entry point (`register_static_table!`
  for builtin, `wit_bindgen::export!` for wasm, `tokio::main`
  wrapping the JSON-RPC loop for process).
- `requires! { "spi.tool" => "1", "spi.secrets" => "1" }` — declares
  capability dependencies inline; checked by the host at load time
  against the manifest.
- `Ctx` — opaque handle exposing only the capabilities the extension
  declared. Methods are typed and category-specific: `secrets()`,
  `http()`, `tracing()`, `emit_event()`. **No untyped
  `host_call(method, params)` escape hatch in v0.1** — that would
  undermine the typed-capability guarantee from R6. Adding a new host
  method is an additive trait extension in `starter-ext-sdk` (and an
  additive capability category in `starter-ext-spi`), not a string
  key. No `&mut self` on anything that reaches `Ctx`.

### `starter-ext-host` (Rust)

The orchestrator. No I/O beyond reading the manifest + static files in
the bundle. Spawning, WASM instantiation, and HTTP serving happen in
the dependent crates.

- `Loader::scan(root: &Path) -> Vec<ExtensionCandidate>` — walks the
  configured extensions directory, parses every `block.yaml`,
  collects errors per-extension without short-circuiting.
- Two-phase commit: `Loader::validate_all()` runs every check (schema,
  namespace, capability compatibility, id uniqueness) on every
  candidate, then `Loader::commit(&mut ExtensionRegistry)` registers
  all-or-nothing. A registry never lands in a partial state.
- `ExtensionRegistry` — `list() -> &[ExtensionRecord]`,
  `get(id) -> Option<&ExtensionRecord>`, `state(id) -> LifecycleState`.
  Immutable after the consumer's `ServerBuilder.build()`.

### `starter-ext-supervisor` (Rust)

Process-flavour lifecycle. The heaviest single crate in the workspace;
gets the most attention because the cost of getting it wrong is the
highest.

- `Supervisor::start(record: &ExtensionRecord) -> SupervisorHandle` —
  spawns the child binary declared in `runtime.bin`, attaches stdio,
  starts the JSON-RPC reader task and stderr-forwarding task,
  performs the init handshake (host sends config; child responds
  ready), transitions the lifecycle state.
- `RestartPolicy` — `always` (restart on any exit), `on_crash`
  (restart only on abnormal exit), `never` (do not restart).
  Intensity cap: `max N restarts within M seconds` → exceeded →
  `Failed`. Backoff: exponential with jitter, capped from the
  manifest. The semantics are the same as Erlang/OTP's
  `permanent | transient | temporary`; the names are renamed for
  readers who have not done supervisor work in Erlang.
- `Health` — periodic `health` JSON-RPC notification with a timeout;
  missed pings count as a crash and trigger the restart policy.
- `EventRing` — bounded ring buffer (default 1000 entries) per
  extension capturing state transitions, crash reasons, restart
  counts, and the last N stderr lines. Surfaced by
  `starter-ext-server` at `/extensions/<id>/events`. Free diagnostics;
  no IO on the hot path.
- Shutdown: `SIGTERM` → grace window from manifest → `SIGKILL`. The
  child's `on_shutdown` runs inside the grace window; the supervisor
  does not block on it past the deadline.

### `starter-ext-wasm` (Rust, OPTIONAL, feature-gated)

WASI-p2 component host on `wasmtime`. Off by default — a consumer
without untrusted-extension requirements does not pay for the wasm
toolchain transitively.

- `WasmHost::instantiate(record: &ExtensionRecord) -> WasmInstance` —
  loads the `.wasm` artefact, applies the manifest's
  `capabilities:` block as WASI grants (default-deny baseline; only
  declared categories are linked into the linker), enforces per-call
  fuel + memory + wall-clock caps.
- One WIT package: `starter:extension@0.1.0`. Authors targeting WASM
  generate bindings from this package; the host links against it.
- Stateless per-call instantiation in v0.1 (matches codeless's
  decision). A `kv` capability for cross-call state is a v0.2
  feature; the WIT seam reserves the import name.

### `starter-ext-server` (Rust)

Axum integration. The wiring crate that turns the host + supervisor
into HTTP routes mountable into a `starter-server::ServerBuilder`.
Same pattern as `starter-mcp`.

- `GET /extensions` — list every extension with id, version, state,
  restart count.
- `GET /extensions/<id>` — full record including manifest, capability
  grants, last health check.
- `GET /extensions/<id>/events` — the supervisor's event ring as a
  paginated JSON response (and SSE upgrade for live tail).
- `POST /extensions/<id>/enable` / `/disable` — runtime toggle;
  disable sends shutdown to the supervisor, enable re-spawns. Both
  gated behind `Authenticator` with `Role::Admin`.
- `GET /extensions/<id>/ui/*` — serves the extension's UI bundle
  (`remoteEntry.js` + chunks) for Module Federation loading. Read
  directly from the bundle dir; cached with strong ETags.

### `starter-ext-ui` (TS)

Host-side Module Federation runtime. **Forked from
`rubix-workspace/extension-ui-sdk`'s `./mf` entry point** with
rubix-specific concepts (graph nodes, kind ids, slot path strings
tied to the graph store) stripped.

- `<ExtensionSlot id="sidebar" theme={mode} themeTokens={tokens}/>` —
  declared slot location; mounts every contribution whose manifest
  sets `slot: sidebar`. `theme` / `themeTokens` are optional and
  thread the host's active mode (`light` | `dark` | custom) and the
  resolved 38-key token map through the per-slot `SlotContext`. Hosts
  wiring `@nube/starter-ui-core/theme-editor` pass
  `useThemeEditorStore(s => s.styles[s.mode])` as `themeTokens`;
  extensions then read them via `useHostTheme()` (see SDK below).
- `useExtensionHost()` — list installed extensions, their state, and
  trigger enable/disable. Reads from `starter-client-ts` against
  `/extensions`.
- `registerExtensionRemote(id, manifestUrl)` — called at app
  bootstrap for each enabled extension. Loads the remote, registers
  shared singletons, exposes mountable contributions.
- Negotiated singletons: React, react-dom, `@tanstack/react-query`,
  zustand. Extensions consume the host's instance; the host enforces
  matching majors.
- **Theme inheritance is automatic for shadcn primitives.** Every
  extension panel mounts inside the host's `<html>`, so the CSS
  variables the host stamps on `:root` (`--background`, `--primary`,
  `--chart-1` …) cascade into the extension's DOM without explicit
  wiring. Extensions only need `useHostTheme()` when they read
  tokens programmatically (chart libraries, canvas, CSS-in-JS).

### `starter-ext-sdk-ts` (TS)

What UI-extension authors import. **Forked from
`rubix-workspace/extension-ui-sdk`'s main entry**, with the rubix
graph hooks (`useNode`, `useSlot`, `useKinds`) dropped — those are
domain-specific to rubix-agent and have no analogue in a generic
starter.

- `useHostClient()` — typed wrapper around `starter-client-ts`
  pre-configured with auth + tracing.
- `<BlockShell>` — standard panel wrapper providing the slot
  context, error boundary, and loading state.
- `useSlotContext()` — hook returning the slot id, host theme mode,
  resolved theme token map, and feature flags relevant to where the
  extension is mounted.
- `useHostTheme()` — the supported read API for the host theme.
  Returns `{ mode, tokens, token(key) }`; the `token()` helper
  prefers the host-supplied map and falls back to live
  `getComputedStyle(document.documentElement)` so the hook works
  even when the host has not wired the theme editor yet. Use this
  for chart palettes, canvas drawings, or any code that needs the
  raw value rather than relying on the CSS-variable cascade.
- `registerExtensionContributions({ slots, components })` — single
  registration entry point called from the extension's `remoteEntry`
  init.

The rubix-ui-kit lift (`@rubix/ui-kit` → `starter-ui-kit`) happens in
the `starter` workspace, not here. Extension authors import shadcn
primitives from `starter-ui-kit` for design consistency.

## The manifest: `block.yaml`

YAML, `deny_unknown_fields`, additive forever within a major. Lives at
the root of every extension bundle. One file; no split metadata.

```yaml
v: 1                                # manifest schema version
id: com.acme.weather                # reverse-DNS; owns com.acme.weather.*
version: 0.1.0
display_name: "Weather"
description_file: docs/README.md    # static markdown; never templated
authors: ["ap@nube-io.com"]

requires:                           # capability dependencies — hard-fail at load
  - { id: starter.spi.tool,    version: "^1" }
  - { id: starter.spi.secrets, version: "^1" }

runtime:                            # exactly one
  kind: process                     # builtin | wasm | process
  bin: dist/weather-driver          # for process: spawned by supervisor
  # crate: com-acme-weather         # for builtin: linked at host BUILD time;
                                    #   manifest is still parsed/validated at
                                    #   startup, but cannot enable/disable code
                                    #   at runtime — only its contributions.
  # artefact: weather.wasm          # for wasm: instantiated by ext-wasm host

supervision:                        # process-flavour only; ignored for builtin/wasm
  restart: always                   # always | on_crash | never
  max_restarts: 5
  within_seconds: 60
  backoff: { initial_ms: 200, max_ms: 30_000, jitter: true }
  health:  { interval_ms: 5000, timeout_ms: 2000 }
  shutdown_grace_ms: 5000

capabilities:                       # what the host grants this extension
  secrets: ["weather:*"]            # SecretStore name prefixes
  http_out: ["api.weather.gov"]     # authority allowlist; wasm-enforced, process-advisory
  fs: []                            # wasm: no fs access; process: advisory
  wall_clock: true

config_schema: schemas/config.json  # JSON Schema for the manifest's `config:` payload
config: {}                          # consumer/operator-supplied values; validated at load

contributes:

  tools:                            # surfaced by starter-ext-mcp AND starter-ext-server
    - id: com.acme.weather.current
      input_schema:  schemas/current_in.json
      output_schema: schemas/current_out.json
      description_file: docs/tools/current.md

  cli:                              # surfaced by starter-ext-cli
    - id: com.acme.weather.fetch    # becomes `<host-bin> weather-fetch [args]`
      subcommand: weather-fetch
      args_schema: schemas/fetch_args.json
      description_file: docs/cli/fetch.md

  rest:                             # surfaced by starter-ext-server
    - id: com.acme.weather.forecast
      method: GET
      path: /weather/forecast       # mounted under host's prefix
      input_schema:  schemas/forecast_in.json
      output_schema: schemas/forecast_out.json
      description_file: docs/rest/forecast.md
      auth: { require_role: Reader }
    - id: com.acme.weather.live     # streaming endpoint
      method: GET
      path: /weather/live
      streaming: sse                # unary (default) | sse | ndjson
      event_schema: schemas/live_event.json
      description_file: docs/rest/live.md
      auth: { require_role: Reader }

  grpc:                             # surfaced by starter-ext-grpc
    - id: com.acme.weather.WeatherService
      proto: proto/weather.proto    # static .proto file; method set is canonical

  workers:                          # surfaced by starter-ext-workers
    - id: com.acme.weather.refresh
      interval_seconds: 300
      jitter_seconds: 30
      on_error: { retry: exponential, max_attempts: 5 }

  ui:                               # surfaced by starter-ext-ui (Module Federation)
    entry: ui/remoteEntry.js
    exposes:
      - { name: WeatherPanel, module: "./Panel", slot: sidebar }
```

Every contribution kind shares the same shape: **`id` (must be in the
extension's namespace), declarative metadata, paths to static
schemas/descriptions, and optional auth/scope/rate-limit knobs**.
None of them carries inline code; the actual handler is the
extension's compiled trait impl.

Every `*_file` and `*_schema` field is a path **relative to the
bundle root**; the loader reads the referenced file at load time and
caches it. `deny_unknown_fields` ensures a typo in a key name is a
load error, not a silent ignore.

## Per-transport contribution mechanics

How a single `ExtensionBehavior` impl reaches every transport. The
mechanics differ per flavour; the *trait the author writes* does not.

**MCP tools** (`contributes.tools` → `starter-ext-mcp`). The adapter
reads the manifest's tools list, constructs a `starter_spi::tool::Tool`
impl per entry, and registers it in `starter-mcp::ToolRegistry`.
Invocation: MCP receives a `tools/call`; registry lookup yields the
extension's Tool; the Tool's `invoke` calls into the extension. For
builtin, a direct function call; for process/wasm, a JSON-RPC
`tool.call` to the child over the existing stdio channel.

**REST routes** (`contributes.rest` → `starter-ext-server`). The
adapter generates one axum handler per entry, applies the declared
auth (`require_role` / `require_scope` from `starter-spi`), and
merges the resulting `Router` fragment into the consumer's
`ServerBuilder`. Path collisions across extensions are a load error.
Bodies and query strings are validated against the entry's
`input_schema` before the extension sees them.

**REST surface for tools** (also in `starter-ext-server`). The same
`contributes.tools` entries are surfaced as `POST /tools/<id>`
endpoints when `starter-ext-server` is mounted — one tool, two
transports, one handler. An operator who wants tools available to
HTTP clients does not need a separate REST contribution.

**CLI subcommands** (`contributes.cli` → `starter-ext-cli`). The
adapter builds a `clap::Command` per entry and registers it in
`starter-cli::CommandRegistry`. The consumer's CLI binary calls
`registry.dispatch(matches)` as it does today; the dispatch routes
extension subcommands through the adapter, which for builtin
extensions calls the trait method directly and for process/wasm
extensions invokes the child over JSON-RPC and waits for the result
synchronously (with a configurable timeout).

**gRPC services** (`contributes.grpc` → `starter-ext-grpc`). Every
`contributes.grpc[]` entry across every loaded extension is reachable
through one backplane service `starter.ext.grpc.v1.ExtensionGrpc`
(`ListMethods` + `Invoke` unary + `InvokeStream` server-streaming),
routed by the manifest's `(service, method)` pair. Arguments and
results travel as canonical proto3 JSON strings — the kernel already
speaks JSON when proxying to process/wasm extensions, so one codec
runs end-to-end. The per-extension `.proto` file in the bundle
remains the schema contract for typed client-side encoding. Typed
dynamic `tonic::server::Grpc` registration (per-extension services
on the same `Server::builder()`) is an additive v0.2 surface under
`starter.ext.grpc.v2`; the v1 backplane never changes once shipped.

**Background workers** (`contributes.workers` → `starter-ext-workers`).
A periodic scheduler invokes each entry at its declared interval,
with jitter and error backoff. Workers are not jobs — there is no
queue, no fan-out, no retry policy beyond the local backoff. A
consumer who needs a real job queue brings their own (R-aligned with
parent SCOPE.md's "not a workflow / job-queue engine" non-goal).

**UI panels** (`contributes.ui` → `starter-ext-ui`). Already covered
under R11. Module Federation; host mounts exposed modules at named
slots; RPC calls from the panel back to the host go through
`useHostClient()`, never directly to extensions.

**Streaming responses are orthogonal to transport.** A handler that
produces a stream of events (AI tokens, log tails, the event ring
live feed, progress updates) is the same handler shape regardless of
how the adapter chooses to render it on the wire. The extension
returns a `Stream<Item = Event>` (mirroring the `OnEvent` channel
shape in `starter-spi::ai`); each adapter renders it natively:

| Transport | Streaming render                                       |
| --------- | ------------------------------------------------------ |
| REST      | SSE frames (or NDJSON if the entry declares it)        |
| CLI       | line-delimited stdout (one event per line)             |
| MCP       | MCP `notifications/progress` messages on the call      |
| gRPC      | server-streaming response (`stream` keyword in proto)  |

The manifest declares streaming per entry:
`streaming: sse` on a REST entry, `streaming: progress` on an MCP
tool, `streaming: stdout` on a CLI command. The default is unary
(single response).

Over the JSON-RPC stdio channel from R10, streaming is a layered
convention: the extension responds to the original request with an
immediate ack carrying a `stream_id`, then emits a sequence of
JSON-RPC notifications (`method: "stream.event"`) tagged with that
`stream_id`, terminating with `method: "stream.end"` (or
`stream.error`). The adapter consumes notifications and translates
them to the transport's native streaming form. **No new wire
format** — same stdio channel, same JSON-RPC framing.

**Cancellation flows back the same way.** Client disconnects (HTTP
close, CLI SIGINT, MCP cancel, gRPC client-side cancel) become a
JSON-RPC `stream.cancel` notification from adapter to extension. The
extension's handler observes cancellation through its `Ctx`'s
cancellation token (mirroring `starter-spi::ai::Cancel`). Extensions
must respect cancellation within a few hundred milliseconds; the
adapter does not wait indefinitely.

For builtin flavour, streams are real `Stream<Item = Event>` values
the adapter drains in-process — no notification framing, no wire
hops. The trait surface is identical; only the wiring differs.

**Authorization is enforced at the adapter, not the extension.**
Each transport adapter applies the auth declaration from the
manifest before invoking the extension. Extensions receive a verified
`Principal` (where the transport has one) but do not perform the
auth check themselves. This keeps auth uniform across surfaces and
prevents an extension from accidentally weakening it.

## Testing seams

Every crate with a non-trivial surface ships a test harness so
consumers and extension authors do not reinvent them. Same discipline
as parent SCOPE.md's testing section.

- **`starter-ext-host::testing::TestRegistry`** — builds an
  `ExtensionRegistry` from inline manifest YAML strings without
  touching the filesystem. Used by host-side tests that drive
  registry behaviour without bundle-layout boilerplate.
- **`starter-ext-supervisor::testing::in_memory_transport()`** —
  returns a paired (host-side, child-side) duplex JSON-RPC channel
  that bypasses process spawning. Tests can drive supervisor state
  machines (init handshake, health, crash, restart, capability
  violation) without spawning a real binary.
- **`starter-ext-sdk::testing::MockCtx`** — a `Ctx` implementation
  that records host calls and returns operator-supplied responses.
  Extension authors unit-test their `ExtensionBehavior` impls without
  a host. Compiles against the same `requires!{}`-generated Ctx
  shape, so type-level capability discipline is preserved in tests.
- **`starter-ext-wasm::testing::ephemeral()`** — instantiates a
  component from in-memory bytes with a minimal default-deny
  capability set; used by host-side WASM integration tests.
- **`starter-ext-server::testing::TestApp`** — wraps
  `starter-server::testing::TestApp` with a pre-configured
  `ExtensionRegistry`; consumer integration tests can hit
  `/extensions` and any `contributes.rest` routes with fixtures.
- **`starter-ext-mcp::testing::tool_pair()`** — pair of (in-memory
  MCP client, MCP server) with an `ExtensionRegistry` mounted so
  `contributes.tools` entries can be exercised end-to-end without
  spawning a subprocess. Builds on `starter-mcp::testing`.
- **`starter-ext-cli::testing::dispatch(matches)`** — invokes the
  `CommandRegistry` against an extension's `contributes.cli` entry
  without spinning up a real binary. Returns captured stdout/stderr.
- **`starter-ext-workers::testing::tick_now(id)`** — fires a worker
  contribution synchronously, bypassing the scheduler, for
  deterministic tests of periodic logic.

On the TS side, `starter-ext-ui/testing` exports
`renderWithExtensionHost(node, { extensions })`, which mounts an
extension shell with `msw` intercepting host-RPC calls. Extension UI
authors render their panels in test isolation; host shell authors
render mock extensions in a real `<ExtensionSlot/>`.

## Smoke tests (before merging anything)

### "One source, three flavours" test

Take the `hello-*` example. Flip its single cargo feature
(`builtin` → `wasm` → `process`). Does it compile, load, and respond
to a tool call in all three modes? If a flavour change requires
touching the `ExtensionBehavior` impl, R1 has slipped.

### "Extension survives host restart" test

Start the host with a process-flavour extension. Send a tool call.
SIGKILL the host. Restart the host. The extension is re-spawned by
the supervisor; the tool call works again. State that the extension
held in `&self` is gone — and that's correct (R5).

### "Crash loop is bounded" test

Start an extension that panics in `on_init`. The supervisor restarts
it according to `always` policy, hits `max_restarts: 5` within 60s,
transitions to `Failed`, and stops restarting. The host stays up and
other extensions are unaffected (R9).

### "Capability violation is rejected, logged, counted" test

A process-flavour extension calls `host.fs.read` without declaring
`fs` in its manifest. The host returns a `capability_violation`
error, logs the attempt with the extension id and the offending
method, and increments the violation counter visible on
`GET /extensions/<id>`. The extension is not killed (it might be
buggy, not malicious); a separate operator policy decides whether to
auto-disable after N violations.

### "Two extensions, no React duplication" test

Two extensions ship UI panels. Both depend on React 18. The host
loads both; the browser DevTools network panel shows one React
bundle, not two. If a second copy of React loads, the singleton
negotiation in `starter-ext-ui` is broken.

### "Bad manifest is isolated to its own extension" test

One extension with a broken manifest fails *itself*, not the host
and not other extensions. The two-phase commit means the registry
never lands in a partial state; the failure is isolated to the bad
extension's record (state: `Failed`, with a parseable reason). Other
extensions load normally. If a typo in `com.acme.weather/block.yaml`
takes down the host or a sibling extension, the loader has a bug.

### "LLM-facing description is byte-identical at load time and call time" test

Take a tool whose manifest declares
`description_file: docs/tools/current.md`. The bytes the host reads at
load time, the bytes surfaced on `GET /extensions/<id>`, and the bytes
the host passes to an LLM or MCP client at call time are
byte-identical. No runtime templating, no substitution, no
extension-mutable text. If an extension can change its own description
after load, R7 has slipped and the anti-prompt-injection guarantee is
gone.

### "Streaming response cancels promptly" test

An extension contributes a REST entry with `streaming: sse` that
emits one event per second indefinitely. A client opens the SSE
stream, receives a few events, and disconnects. Within a few hundred
milliseconds the extension's handler observes cancellation through
its `Ctx` cancellation token and stops emitting. For process flavour,
no JSON-RPC notifications continue to flow after the cancel. If the
extension keeps producing events into a dropped channel, the
cancellation contract has slipped.

### "Same source streams over four transports" test

Take a streaming handler in the `hello-streaming` example. Surface
it as `streaming: sse` over REST, `streaming: stdout` over CLI,
`streaming: progress` over MCP, and server-streaming over gRPC
(`InvokeStream`). The four adapters render the same event
sequence in their respective native forms; the extension code is one
function. If reshaping for a transport requires extension changes,
the streaming abstraction has leaked.

### "Extension author has zero starter-workspace deps" test

Take an extension repository that depends only on `starter-ext-sdk` +
`starter-ext-spi` + `starter-spi`'s public types (`Tool`, errors).
It builds, packages, and loads into a host with **no other starter
or starter-extensions crates** as direct deps. If the SDK has
accidentally re-exported host-internal types, the test fails.

## Non-goals

- **Not a marketplace.** No discovery service, no signed bundles, no
  versioned upstream. Consumers ship extensions out-of-band (a
  filesystem path, a deploy step). Distribution is an orthogonal
  concern.
- **Not hot-reload.** v0.1's `enable`/`disable` requires a clean stop
  and restart of the extension. State carried by the extension is
  lost across cycles. Hot-replace (Erlang code_change-style)
  introduces a `Replacing` lifecycle state and is deferred to v0.2.
- **Not deployment rings.** No canary / stable / promotion model.
  Operators pin an extension version in their deploy; rolling out a
  new version is a redeploy. If a consumer needs progressive
  rollout, they wire it at their deploy layer; not here.
- **Not supervision groups in v0.1.** Every extension is its own
  restart unit. No one-for-all, rest-for-one. The manifest reserves
  a `supervision.group` slot but the supervisor ignores it.
- **Not cgroups / rlimits.** WASM has fuel + memory + deadline caps;
  process extensions are assumed trusted enough that OS-level resource
  isolation is a v0.2 feature behind an explicit threat model.
- **Not multi-tenant.** An extension runs in the host's single trust
  boundary. Per-extension identity / signing / attestation is out of
  scope; consumers who need it write a custom loader.
- **Not a UI design system.** `starter-ext-ui` is a federation runtime,
  not a component library. Visual primitives stay in `starter-ui-kit`.
- **Not a prompt / agent framework.** Extensions contribute `Tool`
  impls (the `starter-spi` MCP-tool seam); they do not contribute
  prompt templates, agent loops, or AI orchestration. Those are
  consumer concerns built on top of `starter-ai` (which is a separate
  starter crate, not an extension).
- **Not a configuration UI.** Extension `config:` values are
  operator-supplied via the manifest. A future "edit extension config
  in the admin UI" feature is a layer on top, not part of the kernel.

## Decisions made (previously open questions)

- **Where it lives:** sibling workspace `starter-extensions/`, not
  inside `starter/`. Preserves SCOPE.md's "small libraries, not a
  framework" line.
- **Manifest format:** YAML, `block.yaml`. Matches rubix; nested
  structures (capabilities, contributes, supervision) read better
  than TOML at this depth. `deny_unknown_fields` catches typos.
- **Manifest schema versioning:** explicit `v: 1` field. New fields
  are additive within a major; breaking changes bump the schema
  version and the loader supports the previous N versions.
- **SDK ↔ host version compatibility:** the manifest's `v:` field is
  the contract. The SDK major bumps lockstep with the schema version
  it produces; an extension built against `starter-ext-sdk = "0.3"`
  emits a manifest with whatever `v:` that SDK supports. A host on
  `starter-ext-host = "0.2"` that only understands `v: 1` rejects
  `v: 2` manifests at load time with a clear "unsupported manifest
  schema" error. The same `v:` field gates the SDK's wire protocol
  expectations for the init handshake; SDK and host negotiate
  capabilities at handshake but the schema version is fixed by the
  manifest.
- **`requires:` vs `capabilities:` semantics:** `requires:` lists
  *categories* the extension needs to function (and host interface
  versions it depends on); the host hard-fails at load if a category
  is required but omitted from `capabilities:`. The *value* of each
  capability entry (allowlist, empty list, scalar) is operator-set
  and scopes the grant: `http_out: []` is a legal "neutralised" grant
  that loads the extension but denies every call at runtime. Operators
  who want the extension absent disable it; operators who want it
  loaded but restricted use scoped grants.
- **Singleton-mismatch handling (UI):** the host enforces matching
  majors on shared singletons (React, react-dom, query lib, store).
  Mismatch is a load-time refusal: the host does not register the
  remote, the extension's lifecycle state goes to `Failed` with
  reason `singleton-mismatch: <pkg>@<expected> vs <actual>`, other
  extensions continue to load. No degraded-mode shell.
- **Builtin trust model:** builtin extensions are trust-equivalent to
  host code. The manifest's `capabilities:` block for a builtin is
  documentation, not enforcement. Operators who need isolation choose
  WASM or process flavour.
- **Stdio JSON-RPC framing crate:** extract `starter-jsonrpc-stdio`
  as a small crate **in the `starter` workspace**, consumed by both
  `starter-mcp` and `starter-ext-supervisor`. Content-Length-framed
  JSON-RPC 2.0 is the same wire format in both worlds; duplicating
  it twice would invite drift. The crate lives in `starter` (not
  `starter-extensions`) because `starter-mcp` is the older consumer
  and the dependency arrow stays inside the parent workspace.
- **IPC wire format:** stdio JSON-RPC 2.0 with Content-Length
  framing (LSP/MCP-style). Not Unix-domain-socket gRPC. Reasons in
  R10. A side-channel for high-throughput cases is additive when
  justified by a real consumer.
- **One trait, three flavours, mutually-exclusive features:**
  compile-time feature guard, linker error on misconfiguration.
  Ported from rubix's `extensions-sdk` pattern (which codeless also
  adopted).
- **Two-phase manifest commit:** validate all candidates, then
  register atomically. One bad extension never poisons the registry.
  Ported from codeless's `PluginRegistry` and rubix's `KindRegistry`.
- **Reverse-DNS ids + namespace ownership enforcement:** ported from
  rubix; codeless also uses prefix matching for persona allowed-tools.
  Kills the entire class of id-shadowing bugs.
- **Stateless behaviours (no `&mut self`):** ported from rubix's
  `NodeBehavior` discipline. Required for flavour interchange.
- **Supervisor restart policy:** three policies — `always` (restart
  on any exit), `on_crash` (restart only on abnormal exit), `never`.
  Intensity cap + exponential backoff with jitter. No supervisor
  groups in v0.1 (R9). The semantics are imported from Erlang/OTP's
  `permanent | transient | temporary`; the *vocabulary* is renamed to
  avoid cargo-culting the Erlang names without the supervision-tree
  context that gives them their original meaning.
- **Event ring for diagnostics:** in v0.1. Bounded ring buffer per
  extension; surfaced via `/extensions/<id>/events`. Diagnostic
  value is large; implementation cost is small.
- **WASM is in v0.1 but feature-gated:** `starter-ext-wasm` is
  optional. Consumers without untrusted-extension needs do not pay
  for `wasmtime` transitively.
- **UI substrate:** Module Federation, host-side singleton
  negotiation. Matches both rubix's and codeless's UI extension
  story.
- **UI package source:** **fork** `rubix-workspace/extension-ui-sdk`
  into `starter-ext-sdk-ts` + `starter-ext-ui`, strip rubix-specific
  graph hooks. **Lift** `rubix-workspace/rubix-ui-kit` into
  `starter-ui-kit` (separate from this workspace; happens in
  `starter/`). **Do not lift** `rubix-workspace/rubix-ui-core` —
  too coupled to rubix's graph model.
- **Three-project audience:** designed greenfield for future starter
  consumers, but every decision is sanity-checked against codeless's
  `plugin.toml` and rubix's `block.yaml` so migration is *possible*.
  Migration is not v0.1's job; the framework does not change to
  accommodate either existing project.
- **Per-transport adapter crates, not a monolithic loader.** Each
  surface an extension can contribute to (MCP tools, REST routes, CLI
  commands, gRPC services, periodic workers, UI panels) gets its own
  small adapter crate, feature-gated. Reasons: (a) a CLI-only
  consumer should not pay for axum transitively; (b) an MCP-stdio
  consumer should not pay for clap; (c) adding a future transport
  (websockets, GraphQL) is a new adapter crate, never a change to
  `starter-ext-sdk` or the trait. The cost is a few more small
  crates; the benefit is that R5 of the parent SCOPE.md
  ("default-features minimal; opt-in everything else") survives the
  extension framework.
- **Adapters apply auth, not extensions.** Each adapter reads the
  manifest's per-contribution auth declaration (`require_role`,
  `require_scope`) and applies it before invoking the extension.
  Extensions receive a verified `Principal` (where the transport
  carries one) but never perform the auth check themselves. This
  prevents extensions from accidentally weakening host security and
  keeps auth uniform across surfaces.
- **Streaming is orthogonal to transport; one convention over
  JSON-RPC.** Handlers return `Stream<Item = Event>` (mirroring
  `starter-spi::ai::OnEvent`). The manifest declares `streaming:` per
  entry (`sse | ndjson | progress | stdout`); adapters render the
  same stream natively per transport. Over the stdio JSON-RPC channel
  (R10), streaming is realised as a notification convention:
  `stream.event` notifications tagged with the parent request's
  `stream_id`, terminated by `stream.end` or `stream.error`.
  Cancellation flows back as a `stream.cancel` notification.
  No new wire format; no new spi trait — the existing `Cancel`
  pattern from `starter-spi::ai` is the model.
- **Extension bundle on-disk convention:** default location is
  `$XDG_DATA_HOME/<binary>/extensions/<id>/` (falling back to
  `~/.local/share/<binary>/extensions/<id>/` when `XDG_DATA_HOME` is
  unset, per the XDG Base Directory Specification). `<binary>` is the
  consumer product's binary name; `<id>` is the extension's
  reverse-DNS id. The consumer can override the root via
  `starter-config` (`extensions.root` key) — the default is just the
  default, never the contract. The loader walks the configured root
  one directory deep; each immediate child whose name matches an
  `id` is a candidate. **Revisit trigger:** a consumer ships on a
  platform without an XDG-equivalent (Windows-only deploy), or a
  consumer needs multi-root layering (e.g. system bundles + user
  bundles merged). Either bumps this from a default to a contract
  shaped by real constraints.
- **Admin-endpoint capability set:** `POST
  /extensions/<id>/enable|disable` ships behind `Role::Admin` from
  `starter-spi` in v0.1. The proposed `Role::ExtensionManage` (or
  `Scope::ExtensionManage`) is **explicitly deferred** — adding it
  speculatively would force consumers to model a role they may never
  need, and removing it later is a breaking change to their RBAC
  config. **Revisit trigger:** the first consumer who wants an
  operator persona that can toggle extensions but cannot perform
  other admin actions. At that point the finer-grained scope is
  added as an *alternative* gate (admin OR extension-manage), not a
  replacement, so existing deployments keep working.
- **`enable`/`disable` persistence model:** disabled state lives in
  a database row keyed by extension id, in a table owned by the
  consumer's storage layer (the same storage seam that backs other
  starter-server admin state). On startup `starter-ext-host` queries
  that table during `Loader::commit` and applies the persisted
  disabled bit to the registry record's `enabled` field; the
  supervisor consults `enabled` before spawning. A sidecar
  `.state.yaml` next to the bundle was rejected because bundles are
  intended to be immutable artefacts (often read-only mounts in
  container deploys); writing mutable state next to them violates
  that. A separate config file was rejected because admin actions
  must be durable across process restarts without a file-write
  step. **Revisit trigger:** a consumer ships without a DB (e.g.
  pure-CLI deploy) and needs a file-backed alternative — the
  persistence layer becomes a small trait (`EnableStore`) with a
  default DB-backed impl plus a file-backed impl, gated by feature.
  The default DB-backed impl is **`starter-ext-store-pg`** — a
  `PgEnablementStore` over `sqlx::PgPool` with a single owned
  migration (`0001_extensions_enablement.sql`) and a testcontainers
  integration test covering get/set roundtrip, UPSERT idempotence,
  `list_all` ordering, and the `updated_by` audit column.
- **JSON-RPC wire-schema versioning via `host_capabilities`:** when
  v0.2 adds its first new host method, the init handshake gains a
  `host_capabilities` field — an explicit set of method names (and
  optional minor versions) the host supports. The extension's SDK
  records this set on the `Ctx` handle; calls to methods absent from
  the set return a typed `Error::Capability("unsupported by host")`
  *before* hitting the wire. Older extensions that never read
  `host_capabilities` are unaffected (they only call v0.1 methods,
  which are always present). Versioning is **per-method capability
  presence**, not a monotonic wire-schema integer — that keeps
  extensions forward-compatible with hosts that add methods, and
  hosts forward-compatible with extensions that ignore new
  capabilities. **Revisit trigger:** the first v0.2 host method
  lands; the handshake field is wired in the same change that adds
  the method, not earlier. (Adding the field with no consumers would
  bake in a shape we cannot validate.)

### Post-R13 follow-ups (adapter / streaming / per-entry auth)

- **JSON-RPC streaming convention lives in `starter-ext-spi`
  alongside `JsonRpcEnvelope`.** Long-running calls (SSE responses,
  CLI tail-style output, gRPC server-streaming, periodic worker
  progress) use a fixed shape: the initial request returns a
  response carrying a `stream_id`; subsequent notifications use
  reserved method names tagged with that id:
  - `stream.event` — one payload chunk for an open stream.
  - `stream.end`   — normal termination (no more events).
  - `stream.error` — abnormal termination (carries an error payload
    matching `Error`'s wire shape).
  - `stream.cancel` — host→extension cancellation request (also
    extension→host if the extension wants to abort an in-flight
    stream it opened).
  Every notification carries `{ "stream_id": "<opaque>", ... }` in
  its params. Adapters (MCP, REST/SSE, CLI, gRPC, periodic, UI)
  translate the streaming shape into their transport's native
  conventions — SSE frames, gRPC server-streaming messages, MCP
  notifications — but the *kernel* shape is one. This belongs in
  `starter-ext-spi` because (a) it is wire-schema (no runtime
  logic), (b) every adapter crate consumes it, and (c) putting it
  anywhere else duplicates the envelope crate's role. **Revisit
  trigger:** an adapter needs back-pressure or flow-control
  signalling that does not fit the four-notification model. That
  promotes streams from "notifications tagged with a stream_id" to a
  first-class lifecycle with its own state machine; v0.1 stays
  simple.
- **Per-entry auth shape in the manifest:** each `contributes` entry
  (tools, ui, future REST/CLI/gRPC entries) carries an optional
  `require_role` and/or `require_scope` field. Both reference types
  defined in `starter-spi` (`Role`, `Scope`) — the same types
  consumer code uses to gate its own routes. **The adapter enforces
  the gate, never the extension.** When a transport-specific adapter
  (e.g. `starter-ext-rest-adapter`) wires an extension entry into a
  route, it wraps the handler with the `Authenticator` middleware
  configured for `require_role` / `require_scope`; the extension
  never sees the request unless the gate passes. This keeps two
  invariants: extensions cannot weaken or skip auth (R6 capability
  discipline), and operators read auth requirements off the manifest
  without reading extension source. Omitting both fields means
  "inherit the adapter's default" (typically `Role::User` for the
  extension namespace, configurable per-adapter). **Revisit
  trigger:** an adapter needs a richer gate than role + scope (e.g.
  ABAC, per-tenant policy). That gate is expressed as a `Scope`
  with parameters, not as a new manifest field — the manifest shape
  is stable.

## Open questions

*(All previously open questions have been resolved; see the
corresponding entries under "Decisions made". This section is kept
deliberately so that future questions land here before being
promoted.)*

- (none — see Decisions for the resolutions of the four previously
  open questions and the two post-R13 follow-ups.)

## Phasing

Each phase is independently mergeable and useful. Stopping after any
phase leaves a working product.

**Kernel phases** (must ship in order):

### Phase 1 — `starter-ext-spi` + `starter-ext-host` + `starter-ext-sdk` (builtin only) + `starter-ext-mcp` adapter

- Trait + manifest + two-phase loader + namespace check + capability
  validation.
- `#[derive(Extension)]` + `requires!{}` macros.
- MCP transport adapter — the simplest adapter, validates the
  contribute-→-adapter pattern end-to-end.
- `examples/hello-builtin` contributes a `Tool` reachable over MCP.

Outcome: end-to-end load of a statically-linked extension into a host
binary, callable from an MCP client. Validates the SPI surface and
the adapter pattern in one shot.

### Phase 2 — `starter-ext-supervisor` + process flavour + `starter-ext-server` admin routes

- stdio JSON-RPC framing + init handshake + bidirectional dispatch.
- Restart policy + intensity cap + backoff + health checks.
- Event ring + stderr forwarding.
- Admin endpoints (`/extensions`, events, enable/disable).
- `examples/hello-process` reuses `hello-builtin` source with one
  cargo feature flipped.

Outcome: out-of-process extensions are first-class.

### Phase 3 — `starter-ext-ui` + `starter-ext-sdk-ts` + bundle serving

- Module Federation host runtime + singleton negotiation.
- `<ExtensionSlot/>`, `useExtensionHost()`, `useHostClient()`.
- `GET /extensions/<id>/ui/*` serves bundles.
- `examples/hello-ui` renders a React panel in a host slot.

Outcome: UI extension story is complete.

### Phase 4 — `starter-ext-wasm` (WASI-p2)

- One WIT package (`starter:extension@0.1.0`).
- Wasmtime instantiation with default-deny capabilities.
- Per-call fuel + memory + deadline caps.
- `examples/hello-wasm`: same source as the other two flavours.

Outcome: untrusted-extension story is complete.

**Adapter phases** (independent; each can ship any time after the
kernel phase its transport depends on):

### Phase 5 — `starter-ext-server` REST adapter

- Surfaces `contributes.rest` as merged `axum::Router` fragments.
- Surfaces `contributes.tools` as `POST /tools/<id>` (one tool, two
  transports).
- Path-collision detection across extensions at load time.
- Auth + tracing layers applied by adapter, not extension.

### Phase 6 — `starter-ext-cli` adapter

- Surfaces `contributes.cli` as `Command` impls in
  `starter-cli::CommandRegistry`.
- Synchronous JSON-RPC dispatch for process-flavour CLI invocations
  (with timeout).
- `examples/hello-cli`: a subcommand contributed by an extension.

### Phase 7 — `starter-ext-workers` adapter

- Tick scheduler with manifest-declared intervals + jitter.
- Per-worker error backoff; no shared queue, no fan-out.
- Surfaces worker state on `GET /extensions/<id>` (last run, last
  error, next due).

### Phase 8 — `starter-ext-grpc` adapter

- Depends on `starter-grpc` (parent workspace) + tonic.
- Surfaces `contributes.grpc` as one tonic backplane service
  `starter.ext.grpc.v1.ExtensionGrpc` (`ListMethods` + unary
  `Invoke` + server-streaming `InvokeStream`), routed by the
  manifest's `(service, method)` pair.
- Builtin dispatch ships end-to-end; process / wasm dispatchers carry
  the `request_timeout` knob and return `UNIMPLEMENTED` until the
  synchronous JSON-RPC dispatch slice lands additively (mirrors
  `starter-ext-cli`'s v0.1 state).
- Streaming follows the kernel's `stream.event` / `stream.end` /
  `stream.cancel` notification convention; client disconnect fires
  the dispatcher's `CancelHandle`.

## Bottom line

**One trait, three flavours, one manifest, one supervisor, one UI
runtime — and one contribution model that reaches every transport
the host exposes (MCP, REST, CLI, gRPC, workers, UI) through small
adapter crates. Designed for the third implementation, not the
first. A consumer's product opts into the adapters it needs; an
extension author writes one struct + one YAML file and reaches every
surface the host has turned on.**
