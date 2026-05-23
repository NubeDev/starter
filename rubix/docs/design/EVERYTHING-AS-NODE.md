# EVERYTHING-AS-NODE — what belongs in the graph, what doesn't

> Source: `rubix/SCOPE.md` R2 ("Observable state is a node. Ephemeral
> state isn't.") and R3 ("The graph is the world. One API, no back
> channels."). Cross-refs: R4 (layer arrow), R7 (extension surfaces),
> R11 (tests).

## The one-sentence rule

> **State that another subsystem or an operator would want to read,
> history, or react to belongs in the graph. Nothing else does.**

R2 is **not** "literally everything is a node." A `tokio::Mutex`
guarding a check-then-act inside one handler is not a node. The flow
engine's per-flow apply-policy buffer is not a node. The HTTP
connection pool is not a node. None of those have a consumer outside
their owning crate.

Promote to a node when at least one of these is true:

- A **flow** would want to branch on the value.
- **Studio** would want to render the value or its history.
- **MCP / gRPC / REST** would want to query the value.
- An **operator** debugging in production would want to read it
  without a debugger attached.
- An **alarm rule** or a **schedule** would want to act on the value.
- **Authz** would want to grant or deny based on the value.

If none of those apply, it's an in-memory field. If you can't answer
confidently, default to **not** a node and promote later when a real
consumer appears. The cost of promoting an in-memory field is one
migration + a behaviour change; the cost of demoting a node that
nothing reads is far higher — slot-history rows, downstream wiring,
tests, and operator muscle-memory all anchor it in place.

## The load-bearing test (apply this every time)

> **Would anyone outside this subsystem — a flow, Studio, MCP, another
> crate, an operator debugging in production — benefit from reading
> this state?**

- **Yes** → it's a node. Define a kind, declare a slot schema, write
  through the graph (R3). No back channel.
- **No** → it's an in-memory field. Keep it private to the owning
  crate. Do not export it; do not surface it on REST; do not put it
  in `rubix-spi`.
- **Don't know** → default to **not** a node. Promote when a consumer
  appears. Premature node-ification is harder to undo than late
  node-ification is to add.

## Concrete examples — these ARE nodes

The Phase 0–4 list, with the consumer that motivates each one.

| Concept | Why it's a node | Reader(s) |
|---|---|---|
| **Device** | An operator commissions it, a flow branches on its online state, MCP lists it. | Studio, MCP, flows, alarms |
| **Point** | A driver writes it, a dashboard renders it, a schedule writes through it, an alarm rule reads it. | Studio, dashboards, schedules, alarms, history |
| **Schedule** | An operator edits it in Studio, a flow reads its next-run, the engine evaluates it. | Studio, engine, audit |
| **Alarm** | Studio renders the inbox; MCP queries unacknowledged alarms; flows react to fired/ack. | Studio, MCP, flows, dashboards |
| **History stream** | Dashboards plot it; MCP exports it; the warehouse mirror reads it. | Dashboards, MCP, warehouse |
| **User** | Studio lists users; authz binds permissions to a user; tenants own users. | Studio, authz, MCP |
| **Tenant** | Resources belong to a tenant; cross-tenant default-deny is keyed off it. | Authz, every domain crate |
| **Team** | A rule grammar `principal.teams contains "ops"` reads it; Studio manages teams. | Authz, Studio |
| **Flow** | The flow editor reads + writes it; the engine runs it; MCP can list flows. | Studio, engine, MCP |
| **Dashboard** | SDUI serves it; authz gates it; operators clone it. | Studio, authz, MCP |
| **Agent health** | An operator-facing status — uptime, queue depth, last warehouse flush. | Studio, ops dashboards, MCP |
| **Setup / enrollment status** | Studio routes off it (first-boot wizard vs steady-state). | Studio, the install flow |
| **Connection state** (driver → external system) | Dashboards render online/offline; alarms fire on disconnect. | Dashboards, alarms |

Each of these has a `KindManifest` (see `KIND-MANIFEST.md`), one or
more typed slots, and a placement rule that says where in the graph
it can live. The graph is the single source of truth: every read and
every write goes through the slot API (R3).

## Concrete examples — these are NOT nodes

| Concept | Why it's NOT a node | Where it lives |
|---|---|---|
| **`tokio::Mutex` guarding a check-then-act** inside one handler | The only reader is the handler that holds it; nothing outside the function needs to see it. | Private field in the handler / domain function |
| **In-process caches** (e.g. `moka::Cache` of recent kind manifests) | Cache contents are an implementation detail; consumers read the underlying authoritative store. | Private field in the registry / domain crate |
| **Connection pools** (Postgres `PgPool`, HTTP `reqwest::Client`) | Transport bookkeeping. Nothing reads the pool's internals; consumers borrow a handle. | Private field in the data crate |
| **Per-request buffers** (decoded JSON body, parsed query string) | Lives one request. No consumer outside the handler. | Stack local |
| **Builders / configurators** (`ServerBuilder`, `RouterBuilder`) | Construction-time scaffolding. No runtime consumer outside the owning crate. | Private mod |
| **One-shot channels / oneshots used internally** | Single producer, single consumer, lifetime = one operation. | Stack local |
| **Tracing spans / metrics counters** | Observed through `starter-observability`, not through the graph. Different audience (R2 §"Observability"). | `tracing::Span`, `prometheus::Counter` |
| **HTTP session tokens** | Transport bookkeeping in `starter-auth-users`. No one outside the auth subsystem needs to read a token. See `AUTH.md`. | `starter-auth-users` tables, opaque cookie |
| **Request-id propagation values** | Plumbing; consumer is the logger, not a flow or dashboard. | Tracing extension |
| **`apply_policy` per-flow buffer** in the propagator | Engine bookkeeping that disappears after the policy resolves. | `starter-flow` internals |

The rule that ties this column together: **transport bookkeeping is
not state. State the platform exposes to its consumers is state.**

## The trap to watch for

If you find yourself about to write a subsystem that owns
`Mutex<SomeObservableState>` that **no slot exposes** — **stop**.
That `Mutex` is a node trying to be born inside a crate that doesn't
yet have a kind for it. Either:

1. Promote the state to a kind with status slots. The `Mutex` goes
   away because the graph is the chokepoint (R3); the slot API
   already serialises writes. Or
2. Confirm via the load-bearing test that nothing outside this
   subsystem needs to read it. If that's true, document why — a doc
   comment citing "no external reader because X" so the next author
   doesn't have to rederive the decision.

The trap is silent: the `Mutex` works, tests pass, and six months
later a second subsystem grows its own copy of the same state because
nobody could see the first one. Now you have two sources of truth and
a divergence bug nobody can debug.

## Edge cases worth calling out

### Setup / enrollment

An agent that has never been configured is in a different state from
a configured agent. Studio renders a different page; CLI gates
commands; the operator's first action is to set this. **That state
is a node** — the `agent.setup` or `agent.enrollment` kind. Reading
it is how Studio decides whether to show the wizard.

### The agent's own health

`agent.health` is a node with slots for uptime, queue depth, last
warehouse flush, last backup, current outbox size. Studio's status
bar reads it; MCP exposes it. **It is not the same surface as
Prometheus metrics**: Prometheus metrics are for platform health
(request latency p99, GC time) consumed by SRE dashboards;
`agent.health` slots are for operator-visible domain state. They
serve different audiences and live on different surfaces — see
SCOPE.md §"Observability" rule 2.

### Driver connection state

A BACnet/MQTT/Modbus driver maintains a TCP connection to an external
broker or device. The TCP socket itself is not a node — but the
**driver's view of whether the connection is up** is. Render
"connected / disconnected / reconnecting" in Studio; fire an alarm on
disconnect; let a flow branch on it. The driver's reconnect timer,
exponential backoff state, and write buffer are private fields.

### Sessions vs users vs tenants vs teams

**Sessions are not nodes.** Nobody outside `starter-auth-users` needs
to read a session token; sessions are transport bookkeeping (see
`AUTH.md`). **Users, tenants, and teams are nodes** because Studio
renders them, flows branch on them, MCP queries them, authz keys off
them.

### Flow runs and Msg envelopes

A flow **definition** is a node (Studio edits it; the engine runs
it). A flow **run** — the in-flight propagation of a single trigger
through nodes — is **not** a node. It is engine bookkeeping. The
`Msg` envelope is immutable on the wire (R6) but ephemeral; produced
by `Msg::new`/`Msg::child`, consumed by the next node. Don't try to
expose `Msg` as a queryable surface; expose the *result* (the slot
write the node performed) instead.

## R3 corollary — one API for state, many verbs for intent

The slot API is the **only** path observable state changes (R3).
**Domain functions** in `domain-*` crates are the **verbs** that
translate intent into slot writes: "commission this device",
"acknowledge this alarm", "publish this dashboard". A transport
handler (R4) extracts a DTO, calls one domain function, and returns;
the domain function performs the slot writes internally through
`graph`.

So: one API for state (slots), many verbs for intent (domain
functions). No transport ever writes a slot directly; no domain
function exposes a private side channel.

The "Swap REST for gRPC" smoke test (OVERVIEW.md) is the cheap
detector for this rule slipping: if a REST handler is doing more than
extract → call domain → return, transport-leak. Move it.

## Where to look in code

- The slot-write chokepoint lives in `agent/crates/graph` —
  `GraphStore::write_slot` is the **only** function that mutates an
  observable slot. Adding a second mutation path is a load-bearing
  rule violation (R3).
- Containment / placement (`placement_allowed(parent_kind,
  parent_manifest, candidate) -> bool`) lives in `graph` as a pure
  function; both `GraphStore::create_child` and REST/CLI handlers
  call it. Never re-implement it transport-side (R4).
- The kind catalogue lives in `agent/crates/kinds-registry`. The
  binary (`apps/agent/src/main.rs`) is the single call site that
  registers every built-in kind at startup.

## Promote-later checklist

If you defaulted to "not a node" and now a consumer has appeared,
promote with these steps:

1. Define the `KindManifest` (`KIND-MANIFEST.md`) — slot names,
   types, value constraints.
2. Add a Postgres migration in `agent/crates/data-postgres` (see
   `MIGRATIONS.md` — `source = "rubix"`).
3. Implement the `NodeBehavior` (`NODE-AUTHORING.md`).
4. Move the in-memory field to write through the graph.
5. Register the kind in `apps/agent/src/main.rs`.
6. Add tests (`TESTS.md`) including a smoke that the previously-
   private state is now readable by an external consumer (REST or
   MCP query, or a flow branch).

The promotion is mechanical because R3 guarantees a single
chokepoint. The expensive part — designing the slot schema — happens
when you write the manifest, and that's the same work you'd avoid by
defaulting to "is a node" prematurely.

## Phase 1 entry expectation

When Phase 1 opens (`domain-devices` + `domain-points`), every public
device/point state must satisfy the load-bearing test above. Driver
internals (reconnect timer, backoff state) are private; driver-
reported online state, configured polling interval, last-read
timestamp are slots. The "Observable state is a node" smoke test in
OVERVIEW.md will be exercised at PR-time on every domain crate; new
`Mutex`/`RwLock` without a slot needs a doc comment justifying why.
