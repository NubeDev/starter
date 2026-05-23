# OVERVIEW — the repo map and dependency arrows

> Source: `rubix/SCOPE.md` §"Repo layout", §"Dependency arrow (Rust)",
> §"Dependency arrow (TypeScript)", R4, R5, R7, R8.
> This doc is the canonical reference a contributor opens first.
> Every other design doc in this directory presumes the map below.

## What `rubix` is

`rubix` is the **product** built on top of `starter` (the platform).
Niagara/Tridium-style: devices, drivers, schedules, alarms, histories,
dashboards — assembled as a graph of nodes on `starter-flow`, served
via `starter-server` (REST + SSE + gRPC + MCP + CLI), authorised by
`starter-authz`, themed and rendered through `starter-ui-kit` +
`starter-ui-core` and a Studio shell.

`starter` ↔ `rubix` is the same shape as `tokio` ↔ `axum`: `starter`
is a dependency, `rubix` consumes it. The directory split is
load-bearing — domain crates **never** leak into `starter/crates/`.

See SCOPE.md "Why this exists separately from `starter`" for the long
form. The TL;DR for a contributor: if you find yourself adding a
device-, point-, schedule-, alarm-, history-, or dashboard-shaped
concept to `starter/crates/`, stop. That concept belongs in `rubix/`.

## Repo map

```
rubix/                                   ← this tree
  SCOPE.md                               ← authoritative design
  mani.yaml                              ← task orchestrator (R13)
  docs/
    design/                              ← THIS DIRECTORY
      OVERVIEW.md                        ← repo map + dep arrows (you are here)
      EVERYTHING-AS-NODE.md              ← R2 in full
      NODE-AUTHORING.md                  ← how to write a NodeBehavior
      KIND-MANIFEST.md                   ← manifest schema + R10 semantics
      AUTH.md                            ← session, JWT, Zitadel, AuthZ
      MIGRATIONS.md                      ← platform/product split for SQL
      TESTS.md                           ← R11 in full
      VERSIONING.md                      ← R10 in full + breaking-change taxonomy
      … (others land just-in-time before the phase that needs them)
    sessions/                            ← working notes (R12)
    testing/                             ← SETUP.md, walkthroughs

  contracts/
    spi/                                 ← rubix-spi (R5: zero internal deps)
    proto/                               ← block.proto + other gRPC schemas

  agent/crates/
    graph/                               ← graph store + propagator (R2 in code)
    engine/                              ← run lifecycle + outbox + graceful shutdown
    kinds-registry/                      ← manifest registration + placement_allowed
    domain-devices/                      ← device commissioning + FSM
    domain-points/                       ← point read/write
    domain-schedules/                    ← schedule eval, calendars
    domain-alarms/                       ← alarm rules, ack/clear
    domain-history/                      ← history ingest, retention
    domain-dashboards/                   ← SDUI dashboard composition
    domain-artifacts/                    ← bundle distribution (Q11a)
    domain-backup/                       ← snapshot/restore
    domain-compute/                      ← transform nodes
    domain-logic/                        ← control-flow nodes
    domain-function/                     ← Rhai Function node
    transport-rest/                      ← axum routes — thin handlers (R4)
    transport-grpc/                      ← tonic; same Tool seam as MCP
    transport-mcp/                       ← starter-mcp bridge
    transport-cli/                       ← clap → agent-client-rs
    data-postgres/                       ← rubix-specific migrations live here
    data-clickhouse/                     ← warehouse adapter (L1/L2/L3)
    data-artifacts-{s3,local,garage}/    ← feature-gated backends
    apps/agent/                          ← the binary; main.rs + registry walk

  agent-sdk/                             ← rubix-extensions-sdk (R7 Rust surface)
  agent-client-rs/                       ← rubix-agent-client (Rust)
  agent-client-ts/                       ← @rubix/agent-client (TS; zero React)
  agent-client-dart/                     ← rubix_agent_client (Dart, mobile)
  ui-kit/                                ← @rubix/ui-kit (Shadcn + tokens; no I/O)
  ui-core/                               ← @rubix/ui-core (the portable brain)
  extension-ui-sdk/                      ← @rubix/extension-ui-sdk (R8 facade)
  studio/                                ← Studio app shell (web + Tauri)
  desktop/                               ← shared Tauri shell (later)
  extensions/                            ← first-party + reference blocks
  repos-cli/                             ← mani-driven workspace tool
```

The directory tree itself encodes most of the rules. R1 enforces file
size (400 lines max, this doc included); R5 puts contracts in one
place with zero internal deps; R7 fences block authors to two named
surfaces; R8 keeps `extension-ui-sdk` as a re-export façade.

## Dependency arrow (Rust)

```
starter-spi
   ↑
rubix-spi                          (R5: contracts hub, zero internal deps)
   ↑
   ├── rubix-extensions-sdk        (R7: block-author Rust SDK)
   ├── rubix-agent-client          (HTTP client; zero agent-crates dep)
   │
   ├── agent/crates/graph
   ├── agent/crates/engine
   ├── agent/crates/kinds-registry
   ├── agent/crates/domain-*       (consume spi + graph + sibling domains)
   │       ↑
   │       └── agent/crates/transport-*
   │              (R4: depends on domain; never reversed;
   │               20-line handler ceiling)
   │                ↑
   │                └── agent/crates/apps/agent
   │                       (binary; only place that knows every
   │                        domain + transport at once)
   └── starter-* (via cargo features the binary chooses)
```

**Never reversed.** No domain crate depends on a transport crate. No
extension crate path-deps `agent/`. `spi` depends on nothing internal.

## Dependency arrow (TypeScript)

```
@rubix/agent-client                (codegen from rubix-spi OpenAPI; zero React)
        ↑
   ┌────┴────┐
   │         │
@rubix/    @rubix/ui-core          (the portable brain;
ui-kit                              hooks/providers/stores)
   ↑              ↑
   │              ├── @rubix/extension-ui-sdk
   │              │           ↑
   │              │           └── extensions/* (third + first party blocks)
   │              │
   └─── studio (the one consuming app shell)
```

`ui-kit` never imports `agent-client`. `agent-client` never imports
React. `extension-ui-sdk` never reimplements what's in `ui-core` — it
re-exports with thin adapters (narrower types, stricter defaults,
named-slot composition over a `ui-core` primitive). R8 is load-bearing
because a bug fix in `ui-core`'s `useNode` must reach Studio, every
block, and every third-party UI **simultaneously**. Two sources of
truth drift silently and the platform rots.

## Where does my code go?

Five common paths; the long form lives in SCOPE.md "Where does my
code go? — the decision tree" (Q1–Q14):

| Change | Lands in | Why |
|---|---|---|
| Wire-level type (`Msg` field, slot key, manifest field, REST DTO) | `contracts/spi` then `mani run codegen` | R5: single contracts hub. **Do not copy types by hand.** |
| Built-in node kind | `agent/crates/domain-*` + register in `apps/agent/src/main.rs` | R3: graph is the world. Read `NODE-AUTHORING.md`. |
| Pluggable block (MQTT, BACnet, …) | `extensions/com.<org>.<name>/` against `agent-sdk` + `extension-ui-sdk` | R7: block-facing surfaces only. |
| REST endpoint | route in `transport-rest`; logic in `domain-*` or `graph`; client surface in `agent-client-*` | R4: 20-line handler ceiling. |
| React hook / provider / API wrapper | `ui-core` | R8: portable brain; **never** in `extension-ui-sdk` or `ui-kit`. |
| Shadcn visual primitive | `ui-kit` | R-K: no I/O, no `agent-client`. |
| Studio page or routing | `studio` | Anything reusable across frontends moves up to `ui-core`. |
| Block UI panel | `extensions/<id>/ui-src/` against `extension-ui-sdk` | R7: never import `ui-core` directly from a block. |
| Agent CLI command | `transport-cli` over `agent-client-rs` | CLI never hits HTTP directly. |
| `mani` task | `mani.yaml` (R13) | If a workflow isn't in `mani.yaml`, add it there first. |

Still unsure? Re-read this doc, then re-read the SCOPE decision tree,
then **ask**. One sentence beats two hours of refactoring the wrong
direction.

## Storage roles — Postgres vs ClickHouse

Two stores with **different jobs**, not a primary/replica split:

- **Postgres** is the system of record. All OLTP: devices, points,
  schedules, alarms, users, tenants, sessions, flows, dashboards,
  permissions. ACID transactions. Owned by
  `agent/crates/data-postgres`.
- **ClickHouse** is the analytical warehouse. Append-only history,
  aggregates, marts. Read-side, eventually consistent — the
  warehouse mirrors a subset of Postgres dimensions
  (one-directional per the warehouse SCOPE). Owned by
  `agent/crates/data-clickhouse`.

A query that **must** be transactional goes to Postgres. A query that
aggregates history goes to ClickHouse. The seam between them is the
warehouse ingest path, not a read-through. **SQLite is forbidden**
across the whole rubix stack (ADR-001 in `starter` extends here).

## The smoke tests (always green before a merge)

From SCOPE.md "Smoke tests":

1. **"Build a new UI"** — delete `studio/`, can someone build a working
   admin UI from `ui-kit` + `ui-core` + `agent-client` on npm alone?
   If no, R8 or R-K slipped.
2. **"Build a new block"** — given only `agent-sdk` +
   `extension-ui-sdk` + `agent-client`, can someone ship a block
   without cloning this tree? If no, R7 slipped.
3. **"Swap REST for gRPC"** — pick a handler; if swapping its
   transport requires rewriting anything but route wiring + DTO
   shaping, domain logic leaked into transport. Move it to `graph` or
   `domain-*`. R4.
4. **"Standalone build"** — a build without `artifacts-*` features
   must boot and serve REST + Studio with no `ArtifactStore` wired in.
5. **"Postgres-only"** — `cargo tree` from `apps/agent` shows zero
   resolution to any `*-sqlite-*` crate. ADR-001.
6. **"Observable state is a node"** — every `Mutex<…>` / `RwLock<…>`
   in `agent/crates/`: does anything outside this subsystem need to
   read it? If yes and the state is not a slot, R2 slipped.
7. **"AI loads context cleanly"** — every file under 400 lines, name
   describes a single concept, test lives at `tests/<name>_test.rs`.
8. **"Comments age well"** — no `// STAGE`, `// Phase`, `// FIXED`,
   `// Previously`, emoji banners, or bare `// TODO` without an owner.

These run in CI and locally via `mani run lint`. The 400-line check
is the bedrock; everything else builds on it.

## What this doc does NOT cover

- **How to write a node** → `NODE-AUTHORING.md`.
- **What is and isn't a node** → `EVERYTHING-AS-NODE.md`.
- **Manifest schema** → `KIND-MANIFEST.md`.
- **Login flow / token handling** → `AUTH.md`.
- **Migration order / source naming** → `MIGRATIONS.md`.
- **Testing patterns + testcontainer convention** → `TESTS.md`.
- **What counts as breaking + how versions bump** → `VERSIONING.md`.

If you've read this doc plus the seven above, you have enough to start
Phase 1 without opening `rubix/SCOPE.md`. The SCOPE is the source of
record for when these docs and reality disagree; reach for it then.

## Phase 0 exit gate

This doc and its seven siblings exist. `mani run build --all` is
green. `mani run lint` is green (R1: no file over 400 lines).
Testcontainer-Postgres and -ClickHouse smoke tests pass against the
empty schema. No domain logic yet — Phase 0 is gate-writing only.

Phase 1 entry gate adds: `AUTH.md` and `MIGRATIONS.md` reviewed and
landed. Both are in this directory.
