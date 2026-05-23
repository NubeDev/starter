# `rubix` — Scope

## One-line summary

`rubix` is the **product** built on top of `starter`. It is a Niagara/
Tridium-style platform for energy, water, and HVAC — devices, drivers,
schedules, alarms, histories, dashboards — assembled as a graph of nodes
on the `starter-flow` engine, served via the `starter-server` transports
(REST + SSE + gRPC + MCP + CLI), authorised by `starter-authz`, themed
and rendered through `starter-ui-kit` + `starter-ui-core` + a Studio
shell.

`starter` is the **platform** — small libraries, opt-in features, no
opinions about the domain. `rubix` is the **product** — opinions,
domain, dashboards, and a binary you can deploy. The boundary between
`starter` and `rubix` is the same shape as the boundary between `tokio`
and `axum`: `starter` is a dependency, `rubix` consumes it.

## Why this exists separately from `starter`

`starter/SCOPE.md` is explicit: *"`starter` is not a framework and not a
template to clone. A consumer does `cargo add` for the pieces they need
and owns their own domain code. The boundary between starter and
consumer is the same shape as the boundary between `tokio` and an app
that uses `tokio`."*

`rubix` is **the canonical consumer**. It exists in this workspace, as a
sibling tree to `starter/crates/`, so that:

1. **`starter`'s API is exercised by a real product**, not by toy
   examples. If a `starter` change breaks `rubix`, that's the signal
   the API is wrong — not a "well, the example was contrived" excuse.
2. **The Niagara-style domain has a home** that isn't crammed into
   `starter`'s plumbing crates. Devices, drivers, schedules, alarms,
   histories, dashboards — these are domain. They live in `rubix/`,
   never in `starter/crates/`.
3. **Third-party blocks have a documented platform target.** A block
   author depends on `rubix-extensions-sdk` and `@rubix/extension-ui-sdk`
   to ship a driver or panel against `rubix`. They never touch `starter`
   directly.

The directory split is **load-bearing**. Putting domain crates into
`starter/crates/` would collapse the platform/product boundary and
every downstream `starter` consumer would inherit Rubix-specific
opinions.

## Hard rules (load-bearing)

These are the rules that make `rubix` extensible by humans and by AI
assistants without the architecture rotting. Break one and the
modularity collapses.

### R1 — One responsibility per file. 400 lines. Always.

This rule applies to **every** language: Rust, TypeScript, Dart.

| Limit | Value |
|---|---|
| Max lines per file | **400** |
| Max lines per function / component | **50** |
| Max public items per module | **~10** |
| Max nesting depth | **4** |

When a file approaches 300 lines, stop and ask: *what are the two or
more responsibilities living here?* Split before 400, not after.

**Name files after the concept they own**, never after their shape.
`password.rs` ✅, `utils.rs` ❌, `common.rs` ❌, `misc.rs` ❌,
`shared.rs` ❌. The rule is **no name-only-by-shape modules** — a name
like `helpers.ts` says nothing about *what* the file contains, only
that the author wasn't sure where to put it; that module grows
forever. (`@rubix/extension-ui-sdk` is *not* a trash drawer despite
being a "facade" — it has a precise scope: the curated re-export
surface for blocks.)

This is the rule an AI assistant must internalise first. A 1200-line
file is a liability — context window burns, edits ripple silently,
nobody can see what's safe to change.

### R2 — Observable state is a node. Ephemeral state isn't.

The rule is **not** "literally everything is a node." The rule is:
**state another subsystem or an operator would want to read, history,
or react to belongs in the graph.**

- **Nodes:** devices, points, schedules, alarms, histories, users,
  flows, dashboards, the agent's own health, setup/enrollment status,
  connection state. Anything a flow should be able to branch on, Studio
  should be able to render, or MCP should be able to query.
- **Not nodes:** request-scoped locks (`tokio::Mutex` guarding a
  check-then-act inside one handler), caches, pools, buffers, builders,
  one-shot channels, transport bookkeeping nobody outside the owning
  crate needs to name.

**The load-bearing test:** would anyone outside this subsystem — a
flow, Studio, MCP, another crate, an operator debugging in production —
benefit from reading this? If yes, it's a node. If no, it's an
in-memory field. If you can't answer confidently, default to *not* a
node and promote later when a consumer appears.

If you find yourself about to write a subsystem owning
`Mutex<SomeObservableState>` that no slot exposes — **stop**, promote
it to a kind with status slots.

### R3 — The graph is the world. One API, no back channels.

Engine, flows, Studio, MCP, CLI, REST, gRPC, extensions — **all** read
and write observable state through the same slot API. When you add a
subsystem, the first question is *"what nodes does it contribute, what
slots does it read/write?"* — never *"what new API do I need?"*

No parallel models, no shadow databases, no "just this once" REST
endpoints that mutate persistent state outside the graph.

**Relationship to R4.** The slot API is the *only* path observable
state changes. **Domain functions** in `domain-*` crates are the
*verbs* that translate intent into slot writes: "commission this
device", "acknowledge this alarm", "publish this dashboard". A
transport handler (R4) extracts a DTO, calls one domain function, and
returns — the domain function performs the slot writes internally
through `graph`. So: one API for state (slots), many verbs for intent
(domain functions). No transport ever writes a slot directly; no
domain function exposes a private side channel.

### R4 — Layer arrow: `contracts → domain → transport`

`spi` (contracts) depends on nothing. `domain-*` crates depend on `spi`.
`transport-*` crates depend on `spi` + `domain-*`. **Never the other
way.** No SQL in handlers, no HTTP in domain, no transport types in
`spi`.

**REST/gRPC/CLI/MCP handlers do four things:** (1) extract inputs, (2)
call a domain function, (3) shape the result into a DTO, (4) return.
**Twenty-line handler ceiling.** If your handler contains business
logic, containment rules, graph walks, or anything that would apply
equally to a different transport — that logic doesn't belong in
`transport-*`. Move it to `graph`, `domain-*`, or a shared crate.

**The canonical smoke test:** *if I swap REST for gRPC tomorrow, how
much of this file changes?* If more than route wiring and DTO shaping
— the layering is wrong.

### R5 — `rubix-spi` is the contracts hub. Zero internal deps.

Wire types live in `rubix-spi`: `KindManifest`, `Msg`, slot schemas,
`block.proto`, REST DTOs decorated with `utoipa::ToSchema`. **Zero
internal deps; zero runtime logic; zero HTTP; zero SQL.** Every other
rubix crate consumes it. TS / Dart / Rust clients codegen from it.

`rubix-spi` depends on `starter-spi` (re-using `Id<T>`, `Error`,
`Principal`, `Page<T>`, etc.) — but **never** the reverse.

### R6 — `Msg` is immutable on the wire

Node-RED parity. Produce new messages via `Msg::new` / `Msg::child`;
don't mutate. The Rhai Function-node is the one place `msg` feels
mutable to the author — the runtime snapshots it on exit (Arc-CoW
semantics; deep-clone only at the boundary that escapes the Function
scope; see `docs/design/NODE-RED-MODEL.md`). Mutation in flight is the
source of races nobody can debug six months later.

### R7 — `rubix-extensions-sdk` and `@rubix/extension-ui-sdk` are the only block-facing surfaces

Extensions (drivers, integrations, custom panels) consume **only**:

- **Rust:** `rubix-extensions-sdk` (the block-author SDK, which itself
  path-deps `rubix-spi` and `starter-spi` only) + `rubix-agent-client`
  (the Rust HTTP client).
- **TS:** `@rubix/extension-ui-sdk` (the curated re-export facade over
  `@rubix/ui-core`) + `@rubix/agent-client` (the TS HTTP client).

Extensions **never** path-dep `rubix/agent/crates/` or
`starter/crates/`, **never** import `@rubix/ui-core` directly,
**never** copy types from `spi`. If the SDK is missing what a block
needs, the fix is to add it to the SDK and bump — never to reach
behind it.

**Pre-stabilisation: in-tree path-deps, registry-equivalent contract.**
While `rubix` lives as a sibling tree to `starter/crates/`, extensions
in `rubix/extensions/` path-dep `agent-sdk` and `agent-client-rs` (and
the TS analogues) by relative path. The path-dep is allowed **only**
to these surfaces. CI enforces, for every crate under
`rubix/extensions/*`:

- `path = "../../agent-sdk"` ✅
- `path = "../../agent-client-rs"` ✅
- `path = "../../contracts/spi"` ✅ (via re-export through agent-sdk;
  direct path-dep on contracts is also acceptable since spi is a
  contract surface)
- any other `path = "../../..."` to a sibling tree ❌ — fail the build.

The same rule applies on the TS side via the workspace's `package.json`
dependency manifest: `@rubix/extension-ui-sdk` and `@rubix/agent-client`
are the only `workspace:*` deps a block may declare.

When `rubix` stabilises and moves to its own registry-published
cadence, the path-deps become registry-version-pinned deps; the
contract above holds unchanged. Extensions written against the in-tree
mode continue to compile against the published mode after a single
`Cargo.toml` / `package.json` substitution.

### R8 — `extension-ui-sdk` re-exports from `ui-core`, never reimplements

Every hook and provider a block consumes has its body in
`@rubix/ui-core`. `@rubix/extension-ui-sdk` re-exports — through a thin
adapter at most — but never re-implements. A thin adapter is
**narrower types**, **stricter defaults**, or **named-slot composition**
over a `ui-core` primitive. Anything else is a parallel implementation
and is forbidden.

A bug fix in `ui-core`'s `useNode` — a missing SSE reconnect, a stale-
cache invalidation, a query-key collision — must reach Studio, every
block, and every third-party UI simultaneously. The moment
`extension-ui-sdk` reimplements, there are two sources of truth and
they will drift silently.

### R9 — Cloud-only deployment. No edge/fleet topology in v1.

`rubix` is a **centralised cloud platform**: one logical agent per
deployment, scaled horizontally if needed, with Studio (web + Tauri
desktop) and the mobile admin (Phase 5) as the operator clients.
There is **no edge/JACE/supervisor split** in v1 — no per-site agent
running disconnected, no edge↔cloud sync layer, no Zenoh fleet bus.

This is deliberate. A multi-tier deployment topology is a 10× scope
increase that v1 does not need to validate the platform/product
boundary. When (and if) a customer requires per-site disconnected
operation, the right answer is to add a `FleetTransport` trait to
`rubix-spi` and a thin transport crate behind it — at which point the
contract is "**domain code is transport-agnostic**, no transport types
in `domain-*`". That seam is *future-proofed but not built*.

No NATS, no Zenoh, no MQTT-as-internal-bus in v1. Block-author MQTT
drivers (the `mqtt-client` extension example) talk MQTT to external
brokers — that is a *driver*, not the platform's internal transport.

### R10 — Versioning is add-only within a major

Contract surfaces — `rubix-spi`, public client APIs, `KindManifest`,
slot schemas, `Msg` shape — version per `rubix/docs/design/VERSIONING.md`.
Within a major, **add-only**. Breaking changes bump the major on the
Rust crate, the npm package, the Dart package, and the agent binary
simultaneously.

### R11 — Tests live with the code

Same PR, same diff. `tests/` mirrors `src/` one-to-one — find the test
for `heartbeat.rs` at `tests/heartbeat_test.rs`. Test-driven where
possible (write the failing test first). Integration tests use the
`starter-server::testing` harness, not hand-rolled servers.

### R12 — Comments explain *why*, never *what*. No session-progress chatter.

Doc-comments on every public item explaining purpose, defaults, edge
cases. No `// STAGE-1 done`, no `// Phase B ✅`, no `// FIXED:`, no
emoji banners, no `// Previously this used Y, now we use Z`. **TODOs
carry a name or ticket:** `// TODO(ap): …` or `// TODO(RUBIX-1234): …`.

**Session-progress chatter has a home — `docs/sessions/`, never source
files.** A working note tracking "stage 4 done, stage 5 next" lives in
a markdown file in `rubix/docs/sessions/` and is invisible to anyone
opening `heartbeat.rs` cold six months later. Source comments describe
the code as it is *now*, in present tense, with no narrative of how it
got here.

When you change behaviour, update the comment in the same diff. A
stale comment is worse than no comment.

### R13 — Drive everything through `mani`

The workspace is multi-repo (or, at this stage, multi-tree). `mani` is
the orchestrator. Three commands daily:

```bash
mani run build --all
mani run test  --all
mani run status --all
```

If a workflow isn't in `mani.yaml`, add it there first. Documentation
and the task file must stay in sync.

## Repo layout

```
rubix/                                   <- THIS TREE
  SCOPE.md                               <- this doc
  mani.yaml                              <- task orchestrator
  docs/
    design/
      OVERVIEW.md                        <- the repo map + dependency arrow
      EVERYTHING-AS-NODE.md              <- R2 in full, with examples
      NODE-AUTHORING.md                  <- how to write a NodeBehavior
      KIND-MANIFEST.md                   <- the manifest schema, versioning
      RUNTIME.md                         <- engine + propagator + outbox
      ARTIFACTS.md                       <- bundle distribution, presign
      BACKUP.md                          <- snapshot / restore lifecycle
      AUTH.md                            <- session, JWT, Zitadel hookup
      QUERY-LANG.md                      <- RSQL on REST/CLI/MCP
      EXTENSIONS.md                      <- block author guide
      VERSIONING.md                      <- R10 in full
      LOGGING.md                         <- tracing + slog integration
      TESTS.md                           <- R11 in full
      UI.md                              <- Studio composition rules
      MCP.md                             <- MCP tool surface
      NODE-RED-MODEL.md                  <- Msg semantics, wires, ports
      HOW-TO-ADD-CODE.md                 <- the session entry doc
    sessions/                            <- working notes, NOT design
    testing/                             <- testing walkthroughs

  contracts/                             <- rubix-spi. R5. Zero internal deps.
    spi/                                 <- Rust crate: KindManifest, Msg,
                                            slot schemas, REST DTOs.
                                            Depends only on starter-spi.
    proto/                               <- block.proto (and any other
                                            gRPC schemas).

  agent/                                 <- the rubix agent binary tree.
    crates/
      graph/                             <- the graph store + propagator.
                                            R2 lives here in code.
      engine/                            <- run lifecycle, state machine,
                                            graceful shutdown, outbox.
      kinds-registry/                    <- KindManifest registration walk,
                                            placement_allowed(), facets.
      domain-devices/                    <- device commissioning + FSM
      domain-points/                     <- point read/write, slot writes
      domain-schedules/                  <- schedule eval, calendars
      domain-alarms/                     <- alarm rules, ack/clear flow
      domain-history/                    <- history ingest, retention
      domain-dashboards/                 <- dashboard composition (SDUI)
      domain-artifacts/                  <- ARTIFACTS.md logic (Q11a)
      domain-backup/                     <- BACKUP.md logic
      domain-compute/                    <- transform nodes (math, etc.)
      domain-logic/                      <- control flow nodes
      domain-function/                   <- Rhai Function node
      transport-rest/                    <- axum routes. Thin handlers (R4).
                                            Uses starter-server.
      transport-grpc/                    <- tonic. Same Tool seam as MCP.
      transport-mcp/                     <- starter-mcp bridge.
      transport-cli/                     <- clap → rubix-agent-client.
      data-postgres/                     <- starter-store-postgres
                                            consumer. Migrations live here.
      data-clickhouse/                   <- warehouse adapter
      data-artifacts-s3/                 <- optional, feature-gated
      data-artifacts-local/              <- optional, feature-gated
      data-artifacts-garage/             <- optional, feature-gated
      apps/
        agent/                           <- the rubix binary.
                                            One main.rs, kind registry walk.

  agent-sdk/                             <- rubix-extensions-sdk.
    src/                                    Block-author Rust SDK.
                                            Path-deps spi + published
                                            transports only. R7.

  agent-client-rs/                       <- rubix-agent-client (Rust).
    src/                                    Zero agent-crates dep
                                            (consumes rubix-spi only).

  agent-client-ts/                       <- @rubix/agent-client (TS).
    src/                                    Zero React. R7.
      generated/                         <- codegen from spi OpenAPI.

  agent-client-dart/                     <- rubix_agent_client (Dart).

  ui-kit/                                <- @rubix/ui-kit.
    src/                                    Shadcn primitives + tokens.
                                            Zero I/O. R-K (HOW-TO-ADD-CODE).

  ui-core/                               <- @rubix/ui-core.
    src/                                    The portable brain. Every
                                            hook/store/provider that talks
                                            to the agent. R8.

  extension-ui-sdk/                      <- @rubix/extension-ui-sdk.
    src/                                    Curated re-export facade.
                                            Re-exports from ui-core.
                                            Never reimplements. R8.

  studio/                                <- the Studio app.
    src/                                    Pages, navigation, Tauri shell.
                                            Consumes ui-core + ui-kit +
                                            agent-client. Owns nothing
                                            another frontend could reuse.

  desktop/                               <- shared Tauri shell (later).

  extensions/                            <- example + reference blocks.
    com.rubix.mqtt-client/               <- block layout template:
      block.yaml                            block.yaml manifest,
      Cargo.toml                            Rust process binary,
      kinds/                                kind YAML definitions,
      process/src/                          MF UI bundle.
      ui-src/

  repos-cli/                             <- the mani-driven workspace tool.
  mani.yaml                              <- task orchestrator
```

## Dependency arrow (Rust)

```
starter-spi
   ↑
rubix-spi
   ↑
   ├── rubix-extensions-sdk      (R7: block-author surface)
   ├── rubix-agent-client        (HTTP client; zero agent-crates dep)
   │
   ├── agent/crates/graph
   ├── agent/crates/engine
   ├── agent/crates/kinds-registry
   ├── agent/crates/domain-*     (every domain crate consumes spi + graph)
   │       ↑
   │       └── agent/crates/transport-*   (R4: depends on domain, never
   │                                       the other way; 20-line handler
   │                                       ceiling)
   │                ↑
   │                └── agent/crates/apps/agent   (the binary; only place
   │                                               that knows about every
   │                                               domain + transport at
   │                                               once)
   └── starter-* (via cargo features the binary chooses)
```

**Never** the other way: no domain crate consumes a transport crate; no
extension crate path-deps `agent/`; no `spi` ever depends on anything
internal.

## Dependency arrow (TypeScript)

```
@rubix/agent-client        (codegen from rubix-spi OpenAPI; zero React)
        ↑
   ┌────┴────┐
   │         │
@rubix/    @rubix/ui-core
ui-kit            ↑
   ↑              │
   │              ├── @rubix/extension-ui-sdk
   │              │           ↑
   │              │           └── extensions/* (third-party + first-party
   │              │                              blocks; depend ONLY on
   │              │                              extension-ui-sdk +
   │              │                              agent-client)
   │              │
   └─── studio (the one consuming app shell)
```

`ui-kit` never imports `agent-client`. `agent-client` never imports
React. `extension-ui-sdk` never reimplements what's in `ui-core`. These
are the load-bearing walls.

## What each crate / package owns

### `rubix-spi` (Rust)

- `KindManifest` — the kind contract: id, version, slot schema, facets,
  permissions, capabilities.
- `Msg` — the immutable wire envelope (R6). `Msg::new`, `Msg::child`.
- Slot schemas — typed slot definitions, `SlotValue` enum.
- REST DTOs — every request/response, `#[derive(ToSchema)]` for OpenAPI.
- `ArtifactStore` trait — for distribution backends (Q11a).
- Re-uses `starter-spi`: `Id<T>`, `Error`, `Principal`, `Page<T>`,
  `Authenticator`, `AiRunner`, `SecretStore`.

### `agent/crates/graph`

The graph store + propagator. Slot writes go through one chokepoint;
the propagator walks downstream. `placement_allowed(parent_kind,
parent_manifest, candidate) -> bool` lives here as a pure function;
both `GraphStore::create_child` and the REST/CLI handlers call it (R4).

### `agent/crates/engine`

Run lifecycle, state machine (lifted from rubix-agent's RUNTIME),
graceful shutdown protocol (SIGTERM → drain in-flight → outbox flush →
exit), session policies (`fresh` / `long-lived`).

### `agent/crates/kinds-registry`

`KindManifest` registration walk; `apps/agent/src/main.rs` calls
`kinds.register(<X as NodeKind>::manifest())` for every built-in kind.
Containment rules, facet bookkeeping, the `placement_allowed` callback
into `graph`.

### `agent/crates/domain-*`

One crate per domain (devices, points, schedules, alarms, history,
dashboards, artifacts, backup, compute, logic, function). Each
crate owns its own kinds, behaviours, slot schemas, FSM, and tests.
Imports `spi` + `graph` + sibling domain crates as needed; **never**
imports a transport crate.

### `agent/crates/transport-rest`

axum routes; thin handlers (R4). Built on `starter-server`'s
`ServerBuilder`. OpenAPI emission via utoipa; `rubix-spi` types
decorated with `ToSchema`. Twenty-line handler ceiling.

### `agent/crates/transport-grpc`

tonic gRPC. Same `Tool` trait seam as `starter-mcp` /
`transport-mcp` — the kinds-registry surfaces the tool catalogue;
gRPC + MCP + REST all consume the same registry. R3.

### `agent/crates/transport-mcp`

Bridge over `starter-mcp`. Tools that surface the rubix domain to
Claude / other agents. Stdio v1; HTTP transport already supported
upstream.

### `agent/crates/transport-cli`

Clap commands over `rubix-agent-client`. The CLI **never** hits HTTP
directly; the client is the abstraction seam.

### Storage roles — Postgres vs ClickHouse

Two stores with **different jobs**, not a primary/replica split:

- **Postgres is the system of record.** All OLTP: devices, points,
  schedules, alarms, users, tenants, sessions, flows, dashboards,
  permissions. ACID transactions. Real-time reads and writes. Owned
  by `agent/crates/data-postgres`.
- **ClickHouse is the analytical warehouse.** Append-only history,
  aggregates, marts. Read-side, eventually consistent — the
  warehouse mirrors a subset of Postgres dimensions (one-directional
  per the warehouse SCOPE) and ingests time-series. Owned by
  `agent/crates/data-clickhouse`.

This is not a contradiction with the "Postgres only" framing
elsewhere in the doc: **SQLite is forbidden** (ADR-001), Postgres is
the OLTP store, ClickHouse is the OLAP warehouse. A query that
*must* be transactional goes to Postgres. A query that aggregates
history goes to ClickHouse. The seam between them is the warehouse
ingest path, not a read-through.

### `agent/crates/data-postgres`

Consumer of `starter-store-postgres`. Owns rubix-specific migrations
(`source = "rubix"` in the namespaced migration runner). **No
SQLite** anywhere in `rubix`. ADR-001 in `starter` extends to the
whole rubix stack.

### `agent/crates/data-clickhouse`

Warehouse adapter; consumer of `starter-store-clickhouse`. L1 raw / L2
curated / L3 marts per the warehouse SCOPE. Tag-driven reads only at
the seam (no raw SQL exposed beyond domain crates).

### `agent-sdk` (`rubix-extensions-sdk`)

The published Rust SDK for block authors. Exposes `NodeBehavior`,
`NodeCtx`, `run_process_plugin()`, slot-event back-channel. Path-deps
**only** `rubix-spi` + `starter-spi` + published transports. **Never**
the internal `agent/crates/*`.

### `agent-client-{rs,ts,dart}`

HTTP clients. `rubix-agent-client` (Rust): used by transport-cli, by
third-party Rust consumers, by integration tests.
`@rubix/agent-client` (TS): zero React, Zod schemas, codegen from
the spi OpenAPI snapshot. `rubix_agent_client` (Dart): for Flutter
mobile admin builds.

### `ui-kit` (`@rubix/ui-kit`)

Shadcn primitives + Tailwind preset + design tokens. Visual-only hooks
allowed (`useViewport`, `useFocusTrap`). **No React Query, no zustand,
no fetches, no `agent-client` import.** Components take data via props.

### `ui-core` (`@rubix/ui-core`)

Every hook / provider / store that talks to the agent. The **portable
brain**. Built on `@tanstack/react-query` + `zustand`. Query keys
prefixed `['rubix', ...]`. Owns `useNode`, `useSlot`, `useHistoryRange`,
`SduiRenderer`, `AuthProvider`, all the rest. **No Studio-branded
pages, no router config** — those belong to the consumer app.

### `extension-ui-sdk` (`@rubix/extension-ui-sdk`)

Curated re-export facade over `ui-core`. R8: re-exports + thin adapters
only; **never** reimplements. Existing in-repo offenders (`useNode`,
`useSlot`, `useAction`, `useSubscription` with bodies inside the SDK)
are technical debt to pay down — not a green light to add more. The
debt is paid down by moving the body into `ui-core` and turning the SDK
entry into a re-export.

### `studio`

The application shell. One consumer of `ui-core + ui-kit +
agent-client`. Owns Studio-branded pages, navigation, the Tauri shell.
**Anything reusable across frontends moves up into `ui-core`** (and
gets re-exported through `extension-ui-sdk` if blocks need it).

### `extensions/`

Reference blocks, first-party. Layout template for third parties:
`block.yaml`, `Cargo.toml`, `kinds/`, `process/src/`, `ui-src/`.

## Cross-cutting concerns

The rules above and the crate map cover *where* code lives. This
section covers the concerns that span every crate — how migrations
order, how identity propagates, how the SDUI surface composes, how
tests run, what triggers a breaking change. Each is a placeholder for
a full design doc; the SCOPE binds the *contract*, the design doc
holds the *implementation*.

### Migrations across the platform/product split

`starter-store-postgres` ships a **namespaced migration runner**: one
`_sqlx_migrations_<source>` table per source, no version-number
collisions. Each component that owns migrations registers its own
source string:

- `starter` — `starter-server` core (request-id, etc., if any).
- `starter_auth_users` — users, sessions, tokens, tenants, teams
  (Phase 7).
- `starter_auth_oauth` — OAuth identities.
- `starter_authz` — policy tables, audit log.
- `starter_prefs` — preferences (already shipped).
- `rubix` — every rubix-specific table (devices, points, schedules,
  alarms, history dimensions, dashboards, …).

**Ordering rule.** Within a startup, migrations run in the order
**`starter_*` first, `rubix` second**. A `rubix` migration may
reference a `starter_*` table (FK into `starter_auth_users_tenants`,
for example); the inverse is forbidden — `starter_*` migrations must
never reference `rubix` tables. CI enforces by parsing the migration
text.

**Rollback rule.** Forward-only. Rollbacks happen by *adding* a new
migration that reverses the prior one — never by editing or deleting
a checked-in file. If a starter migration introduces a constraint
that a rubix migration relies on, removing the starter migration
breaks rubix; the fix is to coordinate the bump (R10) and add a new
migration both sides.

**Detail:** `docs/design/MIGRATIONS.md` (to write).

### Identity, sessions, and Studio→agent auth

Sessions are **not nodes** (R2's load-bearing test: nobody outside
the auth subsystem needs to read a session token). They are transport
bookkeeping in `starter-auth-users` + `starter-auth-oauth`. **Users,
tenants, and teams are nodes** because Studio renders them, flows
branch on them, MCP queries them.

Studio→agent authentication:

- **REST + SSE:** cookie session minted by `starter-auth-users`
  (`POST /auth/login`). `AuthProvider` in `@rubix/ui-core` is the
  React seam.
- **gRPC + MCP:** `Authorization: Bearer <api-token>` — long-lived
  hashed token from `starter_auth_users_tokens`, scoped per Principal.
- **Block process (Rust extension):** the extension supervisor injects
  a per-block bearer token at spawn; the block uses it via
  `rubix-extensions-sdk`'s `Ctx`. Never a user session; always a
  block-scoped credential with explicit scopes from the manifest.
- **CLI:** local token cached by `starter-secrets-keyring` on dev
  machines, env var (`RUBIX_TOKEN`) in CI, or interactive login that
  mints a session.

All four paths resolve to the same `Principal` (`starter-spi`) before
hitting domain code. **No domain function inspects how the principal
authenticated.**

**Detail:** `docs/design/AUTH.md` (to write).

### SDUI, dashboards, per-user gating, i18n, units

Dashboards in rubix are **server-driven UI**: `domain-dashboards`
serves a `UiIr` document (`starter-ui-ir` types); `@rubix/ui-core`'s
`SduiRenderer` renders it. The IR is i18n-keyed and unit-tagged —
the renderer resolves keys via `starter-i18n` and converts
quantities to the active user's units (`starter-prefs`) at render
time. **No locale-specific or unit-specific strings live in the IR
itself.**

Per-user gating is `starter-authz`-driven: a dashboard is a resource
(`ResourceKind::Dashboard`), and `with_permission` wraps the route
that fetches its IR. Phase 7 tenant binding means a dashboard belongs
to a tenant; cross-tenant access is default-deny. Teams can grant
read access via `principal.teams contains "ops"` in the rule grammar.

SDUI surface size and per-user gating make this one of the load-
bearing surfaces of rubix. **Detail:** `docs/design/SDUI.md` (to
write). Phase 3 cannot start without this doc.

### Testing strategy

R11 says "tests live with the code." This section says *how* they
run.

- **Unit tests** live next to source: `src/heartbeat.rs` →
  `src/heartbeat.rs` (Rust `#[cfg(test)] mod tests`) or
  `tests/heartbeat_test.rs` (Rust integration tests; TS / Dart
  follow the same naming).
- **Domain integration tests** use **testcontainers** to bring up
  Postgres (and ClickHouse where the test touches the warehouse).
  `starter-store-postgres::testing::with_database()` is the seam;
  `data-clickhouse` exposes a parallel fixture. A test marked
  `#[ignore = "needs-docker"]` runs only in CI / when `RUBIX_E2E=1`.
- **Transport tests** use `starter-server::testing::TestApp::spawn`
  — random local port, real handlers, real domain functions, real
  database (via testcontainers).
- **Block / extension tests** spawn the block process via the
  supervisor in-test and drive it through `rubix-agent-client`.
- **No mocks at the data layer.** ADR-001's "burned by mock/prod
  divergence" applies; tests against the real store.

**Coverage gate:** to define. Current direction is *no percentage
gate* — a coverage drop is reviewed, not auto-blocked. R11 is a
*presence* gate, not a *coverage* gate.

**R1 enforcement.** `mani run lint` runs `cargo fmt`, `cargo clippy
--all-targets -- -D warnings`, an `eslint` pass on TS, **and** a
line-count check that fails any file under `rubix/` exceeding 400
lines. The line-count check lives in `rubix/repos-cli` (or a tiny
shell script `scripts/check-file-size.sh`) so it can be invoked
locally before push. The smoke test "AI loads context cleanly" is
this check.

**Detail:** `docs/design/TESTS.md` (to write).

### Concurrency, backpressure, and the propagator

`starter-flow` already specifies the propagator shape (one write
chokepoint, ArcSwap topology, per-flow `apply_policy` of `drain |
restart | live-migrate`). `rubix` does **not** re-specify; it
consumes. The flow engine's per-flow concurrency knobs
(`session_policy`, `on_failure`, `safe_state`) are the levers a
rubix-domain author tunes for HVAC-scale point bursts.

What `rubix` adds: **slot writes from drivers are coalesced at the
graph boundary** — bursts to the same slot in a single propagator
tick are merged to the last value before downstream re-evaluation.
This is the only rubix-specific backpressure rule; everything else
defers to `starter-flow`.

**Detail:** `docs/design/RUNTIME.md` (to write).

### Observability

`starter-observability` carries metrics + tracing for rubix. Three
load-bearing rules:

1. **Every domain function emits a tracing span** with the principal,
   tenant, and resource path. The span propagates through `graph`
   into slot writes and downstream nodes — an operator can follow a
   request from REST handler through to the slot write that resulted.
2. **R2 nodes expose status slots; Prometheus exposes platform
   metrics.** Nodes are for *operator-visible domain state* (device
   online/offline, alarm count, schedule next-run); Prometheus is
   for *platform health* (request latency p99, queue depth, GC time).
   They serve different audiences and live on different surfaces.
3. **Trace propagation:** REST handlers extract W3C `traceparent`
   from the request; gRPC uses the standard tonic interceptor; MCP
   wraps the tool call. Tracing context is carried through `graph`
   so a slot-write trace links back to the originating request.

**Detail:** `docs/design/LOGGING.md` (to write — covers tracing,
metrics, and slog integration despite the file name).

### Secrets handling

Block authors will need to read secrets (driver credentials, API
keys, certs). The rule:

- **Block manifests declare required secrets** by stable name
  (`mqtt.broker.password`, `gmail.client_secret`, etc.) in
  `block.yaml`. The supervisor refuses to start a block whose
  required secrets aren't present in the configured `SecretStore`.
- **`rubix-extensions-sdk` exposes `Ctx::secret(name)`** returning
  `Result<Secret, SecretError>` — never the raw store. The SDK
  delegates to whichever `SecretStore` impl the operator wired in
  (`starter-secrets-file` for cloud, `starter-secrets-keyring` for
  dev/desktop).
- **Block-author code never sees the `SecretStore` trait directly.**
  Promote secrets to first-class manifest entries; refuse the
  block-loaded `std::env::var("API_KEY")` antipattern.
- **Rotation:** operator updates the store; the SDK re-reads on next
  `Ctx::secret` call. No process restart required for rotation
  unless the block caches the secret (which the SDK discourages).

**Detail:** `docs/design/SECRETS.md` (to write).

### What counts as a breaking change (R10)

A change to any of these is **major** and bumps every consumer crate
simultaneously:

- **`rubix-spi` Rust API:** removing or renaming a public item;
  changing a function signature; adding a required field to a struct;
  narrowing an enum variant; tightening trait bounds.
- **`KindManifest`:** renaming a slot; removing a slot; changing a
  slot's type; tightening a slot's value constraints. *Adding* a
  slot or an optional manifest field is additive (minor).
- **`Msg` shape:** adding a required field; renaming or removing a
  field; changing a field's type. Adding optional fields is minor.
- **REST DTO:** removing or renaming a field; changing a field's
  type; tightening validation. Adding optional fields is minor.
- **`block.yaml` schema:** any change requiring existing manifests
  to be edited.
- **`starter-spi` re-exports:** if rubix re-exports a starter type
  and starter bumps it, rubix bumps too.

Non-breaking: adding a new endpoint, adding a new kind, adding an
optional field, adding a new domain function, adding a new error
variant marked `#[non_exhaustive]`.

**Detail:** `docs/design/VERSIONING.md` (to write).

## Where does my code go? — the decision tree

Walk top to bottom. Stop at the first "yes".

### Q1. Am I changing a wire-level type?

*Examples: field added to `Msg`, new slot-schema key, new facet, new
`KindManifest` field, new REST DTO.*

→ **`rubix/contracts/spi`**. Then `mani run codegen` to regenerate
`agent-client-ts/src/generated/`. Every downstream consumer
(`agent`, `agent-sdk`, `agent-client-rs`, `ui-core`) picks it up on
next rebuild. **Do not copy types by hand.**

### Q2. Is this a built-in node kind that ships in the agent binary?

*Examples: `sys.logic.function`, `sys.compute.pluck`, `sys.alarm.rule`.*

→ A `domain-*` crate under `agent/crates/`. Pick the matching domain
(`domain-logic` for control flow, `domain-compute` for transforms,
`domain-alarms` for alarm kinds, etc.); create a sibling crate if the
concern is new. Register the kind in `agent/crates/apps/agent/src/main.rs`.
Read `docs/design/NODE-AUTHORING.md`.

### Q3. Is this a pluggable block (user-loadable, possibly third-party)?

*Examples: MQTT, BACnet, Modbus, a project-management block.*

→ `extensions/com.<org>.<name>/` — standalone Cargo crate + optional MF
UI bundle. Consumes `rubix-extensions-sdk` (published). **Do not**
path-dep `agent/`. If the SDK is missing something, **add it to the
SDK and bump** — never reach behind.

### Q4. Is this a REST endpoint on the agent?

*Examples: `GET /api/v1/kinds`, `POST /api/v1/devices`,
`GET /api/v1/history`.*

→ **Route in** `agent/crates/transport-rest`.
→ **Domain logic in** `domain-*` or `graph` (R4; never inline in the
   handler).
→ **Client surfaces in** `agent-client-rs`, `agent-client-ts`,
   `agent-client-dart`.

For filter/sort/pagination, use the RSQL query framework
(`docs/design/QUERY-LANG.md`) — one `QuerySchema`, every transport
(REST, CLI, MCP) gets the filter for free.

### Q5. Is this a React hook, store, provider, or API wrapper?

*Examples: `useKinds()`, `useNode(path)`, `AuthProvider`, `SduiRenderer`.*

→ **`ui-core`**. The portable brain. Studio, mobile admin, and (via the
re-export through `extension-ui-sdk`) blocks all consume the same hook.

**Never** put hooks in `ui-kit`. **Never** author hook bodies in
`extension-ui-sdk` (R8).

### Q6. Is this a visual primitive — Shadcn component, design token, icon?

*Examples: Button, Badge, Select, Dialog, Card, Tailwind preset.*

→ **`ui-kit`**. Pure visual primitives only. **No** React Query, no
zustand, no `agent-client`, no I/O hook. If the component needs data,
it takes it as a prop.

### Q7. Is this a Studio page, feature, or app-level routing?

*Examples: DevicesListPage, AlarmInboxPage, SettingsPage, router config.*

→ **`studio`**. The app shell. Imports `@rubix/ui-kit`,
`@rubix/ui-core`, `@rubix/agent-client`.

**Never** put Studio-specific pages in `ui-core`. If a page is reusable
across frontends (a generic node editor, an alarm-inbox skeleton), it
can live in `ui-core/src/pages/` — but Studio-branded navigation, the
Tauri shell, and brand assets stay in `studio`.

### Q8. Is this a block UI panel (MF bundle inside a block)?

*Examples: mqtt-client Panel, bacnet Panel, project-management Kanban.*

→ **`extensions/<id>/ui-src/`**. Depend **only** on
`@rubix/extension-ui-sdk` and `@rubix/agent-client`. **Never** import
`@rubix/ui-core` directly from a block. If something is missing in
the SDK, add the re-export there (and, if the implementation is
missing, add it to `ui-core` first).

### Q9. Is this the `agent <cmd>` CLI?

*Examples: `agent kinds list`, `agent slots write`, `agent devices ls`.*

→ **`agent/crates/transport-cli`**. Thin clap wrapper over
`rubix-agent-client`. If the command needs a capability the client
doesn't expose, **add it to `agent-client-rs` first**, then call from
the CLI. The CLI never hits HTTP directly.

### Q10. Is this the Tauri desktop shell?

*Examples: native menu bar, system tray, auto-updater hook, FS access.*

→ **`desktop`**. The shared `src-tauri` backend.

### Q11a. Is this artifact distribution (object store, presigned URLs, bundle upload/fetch)?

*Examples: `ArtifactStore` trait, S3/Garage/local backend, presigner
endpoint, block-install fetch, snapshot upload, local cache eviction.*

→ Trait in **`contracts/spi`**.
→ Pure logic in **`agent/crates/domain-artifacts`**.
→ Backends in sibling crates gated by Cargo features
   (`data-artifacts-s3`, `data-artifacts-local`,
   `data-artifacts-garage`). **Compiled out by default** — standalone
   builds ship without object-store I/O.
→ Thin REST handlers in **`transport-rest/src/artifacts.rs`** (≤20
   lines each, R4). **Registration is feature-gated** — in a standalone
   build with no `artifacts-*` feature on, the routes do not exist.

Read `docs/design/ARTIFACTS.md` before writing any of this — the
control-plane / data-plane split is load-bearing and easy to get wrong.

### Q12. Is this MCP work?

*Examples: new MCP tool, prompt-injection mitigation, stdio auth.*

→ **`agent/crates/transport-mcp`** for the tool surface. The kinds
registry already exposes node kinds as tools; new bespoke tools register
the same way.

### Q13. Is this orchestration / dev tooling / `mani` task?

*Examples: new `mani run` task, new tree in the workspace, version
pinning.*

→ **`repos-cli`** for the tool; **`mani.yaml`** for tasks.

### Q14. Is this documentation?

- **Design / architecture** → `rubix/docs/design/` (this is the
  authoritative tree).
- **Session / working plan** → `rubix/docs/sessions/`.
- **Testing walkthrough** → `rubix/docs/testing/`.

### Still unsure?

→ Read `docs/design/OVERVIEW.md` for the full map + dependency arrow.
   Then **ask**. One sentence beats two hours of refactoring the wrong
   direction.

## Smoke tests (before merging anything)

### "Build a new UI" test

If someone deletes `studio/` entirely and only has `@rubix/ui-kit` +
`@rubix/ui-core` + `@rubix/agent-client` on npm, can they build a
working admin UI? **If your change requires them to fork or reach into
Studio's code — you broke R8 or R-K. Put the reusable piece in
`ui-core`, the primitive in `ui-kit`, and only Studio-specific
composition in `studio`.**

### "Build a new block" test

If someone has only `rubix-extensions-sdk` + `@rubix/extension-ui-sdk`
+ `@rubix/agent-client` on their registries, can they build and ship a
block without cloning this tree? **If your change requires them to
path-dep `agent/`, copy types from `spi`, or import `@rubix/ui-core`
directly — you broke R7 or R8. Put the Rust capability in `agent-sdk`,
the TS capability in `extension-ui-sdk` (re-exporting from `ui-core`),
and nothing block-specific in either.**

### "Swap REST for gRPC" test

Pick a handler in `transport-rest`. If swapping its transport would
require rewriting anything other than route wiring and DTO shaping,
domain logic leaked into transport. Move it to `graph` or `domain-*`.
Run the same test on every `transport-*` crate.

### "Standalone build" test

A standalone agent compiled without the `artifacts-*` features must
build, boot, and serve REST + Studio with no `ArtifactStore` or
related routes wired in. If any of those bleed into a non-optional
path, R-feature-gating slipped.

### "Postgres-only" test

A `cargo tree` from `agent/crates/apps/agent` must show **zero** path
or `crates.io` resolution to `starter-store-sqlite` or any
`*-sqlite-*` crate. ADR-001 extends to the whole rubix stack.

### "Observable state is a node" test

Walk every `Mutex<…>` / `RwLock<…>` in `agent/crates/`. For each, ask:
*does anything outside this subsystem need to read this?* If yes and
the state is not exposed as a node slot — R2 slipped, promote the
state to a kind.

### "AI loads context cleanly" test

Pick any file in `rubix/`. Is it under 400 lines? Does its name
describe a single concept (`heartbeat.rs`, not `utils.rs`)? Does its
test live at `tests/<same_name>_test.rs`? If any answer is no — R1
slipped, split before merging.

### "Comments age well" test

Grep the diff for `// STAGE`, `// Phase`, `// FIXED`, `// Previously`,
emoji banners, and bare `// TODO` without an owner. Any hit fails R12.

## Non-goals

- **Not a framework.** `rubix` does not invite consumers to extend by
  inheritance or by editing a generated tree. Third parties extend
  through `agent-sdk` and `extension-ui-sdk` only.
- **Not multi-tenant by accident.** Tenants and teams are first-class
  in `starter-authz` Phase 7 (already shipped). `rubix` consumes them;
  no shadow tenancy model lives in `rubix/crates/`.
- **Not an SSO provider.** `rubix` consumes `starter-auth-users` (local
  users + sessions + tokens) + `starter-auth-oauth` (GitHub / Google).
  Zitadel hookup is documented in `docs/design/AUTH.md`. OIDC consumers
  swap the `Authenticator` behind the trait; the seam doesn't move.
- **Not a workflow engine.** Flows are graphs in `starter-flow`. If a
  consumer needs background jobs separate from the graph, they bring
  their own job runner.
- **Not an AI orchestration brain.** `starter-ai` is the provider seam
  (Claude / Codex / Copilot CLIs, Anthropic / OpenAI REST). Agent
  orchestration is graph topology in `starter-flow`; `rubix` adds
  domain kinds, not a parallel AI runtime.
- **Not SQLite-aware.** Postgres only. ADR-001 in `starter` is binding.
- **Not multi-tier in v1.** R9 — cloud-only, one logical agent per
  deployment. No edge/supervisor topology, no fleet bus, no per-site
  disconnected operation. The seam is future-proofed in `rubix-spi`
  but not built.
- **Not a place to put parallel `ui-core` implementations.** R8 is
  load-bearing.

## Decisions made (locked)

- **Directory location:** `rubix/` is a sibling tree to
  `starter/crates/` inside the same git repo (one repo, two trees —
  not two repos). When `rubix` stabilises, it can move to its own repo
  consuming `starter` as a registry dependency; until then, the
  in-repo path-dep accelerates iteration without breaking the
  platform/product boundary.
- **Postgres only.** No SQLite anywhere in `rubix/`. Extends ADR-001.
  ClickHouse is the analytical warehouse (read-side), Postgres is the
  system of record (OLTP) — see "Storage roles — Postgres vs
  ClickHouse" for the contract.
- **Languages:** Rust for the agent, TS/React for Studio + UI libs,
  Dart/Flutter for the mobile admin client. No Go in `rubix/` at this
  time.
- **Cloud-only in v1 (R9).** One logical agent per deployment, scaled
  horizontally if needed. No edge/JACE/supervisor split, no fleet
  bus, no `rubixd` supervisor daemon. The seam is future-proofed in
  `rubix-spi` (a `FleetTransport` trait can be added later without
  domain-code changes) but not built. Deferred to a post-v1 track.
- **Studio targets at launch:** web + Tauri desktop. Flutter mobile
  admin lands in Phase 5 against `agent-client-dart`. No mobile React,
  no native Android/iOS for Studio itself.
- **Transport:** axum for REST, tonic for gRPC, stdio for MCP, clap
  for CLI. No internal messaging bus in v1 (R9).
- **Publishing model (R7):** in-tree path-deps to `agent-sdk`,
  `agent-client-rs`, and `contracts/spi` only — CI-enforced. Registry
  publishing comes when `rubix` stabilises; the contract holds
  unchanged across the cutover.
- **Migrations order:** `starter_*` migrations first, `rubix` second.
  Forward-only; rollbacks happen by adding a reversing migration, not
  by editing the prior one. See "Migrations across the
  platform/product split".
- **`extension-ui-sdk` debt:** existing in-SDK hook bodies (`useNode`,
  `useSlot`, `useAction`, `useSubscription`) are technical debt. Pay
  down incrementally: when you touch one, move the body to `ui-core`
  and turn the SDK entry into a re-export. Do not add new offenders.
- **i18n:** English + Spanish from Phase 1 — every feature ships with
  both catalogues populated at PR time, not at Phase-5 catchup.
  `starter-i18n` ships the framework; `rubix` owns the catalogues.
- **User prefs:** units (metric/imperial) + time-format + theme +
  number-format + date-format + week-start. `starter-prefs` already
  ships the resolver; `rubix` consumes it. Per-user and per-tenant
  overrides — never a global flag.
- **AuthZ:** `starter-authz` Phase 7 — tenants + teams + decision
  audit — is binding for `rubix`. No parallel RBAC. Sessions are
  *not* nodes; users, tenants, teams *are* nodes.
- **R1 enforcement:** `mani run lint` includes a 400-line file-size
  check. PRs with a file over 400 lines fail CI.

## Open questions

- **Block hot-reload contract.** `starter-flow`'s hot-reload (HR1–HR8)
  already handles process / builtin / WASM flavours. Confirm
  `rubix-extensions-sdk` exposes the right ergonomics for block authors
  before freezing the surface.
- **Block-process trust boundary.** A block runs in its own process
  (R3 of extensions SCOPE) and receives a per-block bearer token. Are
  there blocks the platform should refuse to load even with operator
  consent (e.g. ones requesting `*` permissions)? Tied to the AuthZ
  Phase 7 audit surface.
- **Coverage gate.** Is presence-of-tests (R11) enough, or do we want
  a percentage gate at some Phase? Easy to add later; harder to
  remove.

## Phases (rough)

Phases are strictly ordered; each assumes the previous landed. **Each
phase has explicit entry gates** — design docs that must exist before
the phase opens. Skip a gate and the phase will rot.

### Phase 0 — Lay the trees + write the load-bearing design docs

Entry gate: none (this is the gate-writing phase).

- Create `rubix/contracts/spi` (empty crate, depends on `starter-spi`).
- Create `rubix/agent/crates/{graph, engine, kinds-registry}` skeletons.
- Create empty `agent-sdk`, `agent-client-rs`, `agent-client-ts`,
  `agent-client-dart` (skeleton only), `ui-kit`, `ui-core`,
  `extension-ui-sdk`, `studio`, `desktop`.
- Wire `mani.yaml` for build / test / status / lint across the new
  trees. The lint task runs the 400-line file check (R1).
- Wire testcontainers-based Postgres + ClickHouse fixtures via the
  `starter-store-*::testing` seams.
- Write the load-bearing design docs that subsequent phases reference:
  `OVERVIEW.md`, `EVERYTHING-AS-NODE.md`, `NODE-AUTHORING.md`,
  `KIND-MANIFEST.md`, `AUTH.md`, `MIGRATIONS.md`, `TESTS.md`,
  `VERSIONING.md`. (Other docs land just-in-time before the phase
  that needs them.)
- Exit: `mani run build --all` green; `mani run lint` green;
  testcontainer-Postgres integration test passing for the empty
  schema. No domain logic yet.

### Phase 1 — Devices + points + i18n catalogues + Studio shell

Entry gate: Phase 0 exit + `AUTH.md` + `MIGRATIONS.md` written.

- `domain-devices` + `domain-points` (in-process; **no driver
  protocol yet** — a "device" in Phase 1 is a model in the graph
  with operator-writable slots, not a physical sensor).
- REST + CLI + MCP surfaces for both, with the matching client
  methods in `agent-client-rs` and `agent-client-ts`.
- Studio shell up: web + Tauri desktop, both connecting to a cloud
  agent. Studio router, theme switcher, `AuthProvider`, devices list
  page, device detail page.
- Postgres migrations under `data-postgres` for devices, points,
  and any rubix-side users-tenants extensions.
- **English + Spanish catalogues for every Phase-1 message key.**
  i18n is not a Phase 5 concern; it ships per-feature from Phase 1.
- Exit: a user logs in, lands in Studio (web or desktop), commissions
  a device, writes a point's value, sees the slot update reflected
  in MCP and CLI. English and Spanish are switchable in the UI.

### Phase 2 — Schedules + alarms + history

Entry gate: Phase 1 exit + `RUNTIME.md` written.

- `domain-schedules`, `domain-alarms`, `domain-history`.
- ClickHouse hookup via `data-clickhouse` for history ingest +
  warehouse reads.
- Alarm inbox page in Studio (page in `studio`, hook in `ui-core`).
- Schedule editor (page in `studio`, calendar primitive in `ui-kit`).
- Exit: a scheduled action writes a slot at the right time; an alarm
  rule fires on a slot threshold; history is queryable through the
  warehouse for the last 30 days.

### Phase 3 — Dashboards (SDUI) + first extension

Entry gate: Phase 2 exit + `SDUI.md` + `EXTENSIONS.md` written.

- `domain-dashboards` consumes `starter-sdui-routes`. SDUI renderer
  in `@rubix/ui-core` resolves i18n keys and unit-tagged quantities
  at render time.
- Per-user / per-tenant dashboard gating via `starter-authz`.
- `extensions/com.rubix.mqtt-client/` as the reference block —
  process binary against `rubix-extensions-sdk`, MF panel against
  `@rubix/extension-ui-sdk`. Validates the block contract end to end.
- Extension installer + supervisor (consumes `starter-extensions`).
- Exit: an operator builds a dashboard in Studio; a second operator
  in a different tenant cannot see it; the MQTT block, installed from
  the block library, surfaces an external broker's topics as points.

### Phase 4 — Artifacts + warehouse marts + production hardening

Entry gate: Phase 3 exit + `ARTIFACTS.md` written.

- `domain-artifacts` complete — block bundle distribution via the
  object-store backends (S3, Garage, local). Feature-gated; standalone
  builds skip.
- Warehouse marts (L3) for the standard rubix domain: device-uptime,
  alarm-rate, energy-by-meter, etc.
- Observability hardening: tracing across REST→domain→slot, Prometheus
  metrics on the standard surface, slog catalogue for operators.
- Backup/restore via `domain-backup`.
- Exit: a customer can deploy a hardened cloud agent, install
  third-party blocks, run a dashboard pack against their data, and
  back the whole thing up nightly.

### Phase 5 — Mobile admin

Entry gate: Phase 4 exit + `agent-client-dart` v1 stable.

- Flutter mobile admin consuming `agent-client-dart` directly (no
  React stack on mobile; Dart only).
- Mobile feature set: device list, alarm inbox, point read/write,
  user prefs. Not dashboards (SDUI renderer is React-only in v1).
- i18n in mobile from day one (English + Spanish), units honoured.

### Out of scope for v1 (post-release tracks)

- **Multi-tier deployment.** Edge agents, fleet bus, per-site
  disconnected operation. Requires a `FleetTransport` trait + a
  transport crate behind it (see R9 future-proofing).
- **Supervisor daemon (`rubixd`).** A/B updates, OTA, systemd. Only
  meaningful in a multi-tier world; deferred with the fleet topology.
- **Additional block templates.** Beyond the MQTT reference, blocks
  for BACnet, Modbus, etc., are downstream consumer concerns.

## Bottom line

**`starter` is the platform. `rubix` is the product. One graph, one
slot API, one set of clients, one curated SDK facade for blocks. AI
assistants and humans build on it the same way: walk the decision
tree, respect the layer arrow, keep files small, promote observable
state to nodes, run `mani run build --all` before committing, and
ask before crossing a boundary you weren't sure about.**
