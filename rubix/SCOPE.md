# `rubix` — Scope (backend-only)

## One-line summary

`rubix` is the **backend product** built on `starter`. One binary that
wires every `starter-*` capability and ships a curated set of **tools,
skills, and flows** for: building dashboards, managing users,
programming flows, writing ClickHouse rules, running background jobs,
and producing analytics reports — all driven by the `ai-agent` node
kind from starter, all exposed over REST + SSE + gRPC + MCP + CLI.

Backend product first; UI surfaces ([`rubix/frontend/`](frontend/)
and the planned [`rubix/mobile/`](docs/scope/mobile/)) consume the
backend per [ADR 0004](docs/adr/0004-react-native-mobile-app.md)
and never the reverse.

## The mental model

`starter` already owns the agent runtime, the flow engine, the kind
registry, the tool trait, the skill format, the extension host, every
transport, and every store. **`starter`'s [DOCS/agent/SCOPE.md](../DOCS/agent/SCOPE.md)
is the authoritative agent spec — read it before this one.** The key
facts from that doc that shape rubix:

- **"An agent" = a flow rooted at an `ai-agent` node.** Multi-agent
  topology is flow topology. There is no `Agent` Rust type.
- **`AiRunner` is the only LLM seam.** CI-enforced via `cargo tree`.
- **Skills are `SKILL.md` files**, content-hash quarantined, owned by
  `starter-skills`.
- **Every flow auto-surfaces as an MCP tool** via `FlowAsTool` — no
  wiring.
- **Extensions contribute tools / skills / flows / nodes** via
  `block.yaml` through `starter-ext-flow`. No bespoke host.

Rubix's job is therefore not to *build* an agent runtime, but to
**ship the domain content** the rubix agent uses:

1. **Tools** the agent dispatches (`rubix-tools`).
2. **Skills** that gate and steer the agent (`rubix-skills`).
3. **Flows** that compose the tools into the six rubix goals
   (`rubix-flows`).
4. **One binary** that composes everything (`rubix-agent`).

Plus the contracts and a client.

## The six rubix agent goals (load-bearing)

The rubix backend exists to serve these six goals. Every tool, skill,
and flow we ship maps onto one of them. Anything that doesn't is
out of scope.

| # | Goal | Tools (rubix-tools) | Bundled flow |
|---|---|---|---|
| 1 | Build / program dashboards (SDUI) | `dashboard.{create,update,list,page.set}` | `flows/dashboard-assistant.yaml` |
| 2 | Manage users + teams + tenants | `user.{create,disable}`, `team.{create,assign}`, `tenant.list` | `flows/user-admin.yaml` |
| 3 | Program flows (build / edit / deploy) | `flow.{deploy,validate,lint,list}` | `flows/flow-programmer.yaml` |
| 4 | Write / manage ClickHouse rules + marts | `clickhouse.{rule.write,mart.create,retention.set}` | `flows/clickhouse-ruler.yaml` |
| 5 | Background jobs / system checks | `system.{disk,db,flow-errors}`, `alert.send` | `flows/scheduled-system-check.yaml` (cron) |
| 6 | Analytics + reports | `analytics.{query,report}` | `flows/weekly-report.yaml` (cron) |

**MCP goal:** every one of those flows is automatically an MCP tool
(per starter's `FlowAsTool`). Other agents — Claude Desktop, another
rubix instance, any MCP client — connect to rubix and see the six
goals as callable tools. **Zero MCP wiring code in rubix.**

## What rubix is, exactly

A single backend that exposes:

1. **Auth + authz** — local users, OAuth, sessions, tokens, tenants,
   teams, decision audit. From `starter-auth-users` +
   `starter-auth-oauth` + `starter-authz`.
2. **Flow runtime + `ai-agent` node kind** — the agent loop, kind
   registry, propagator, hot reload, sessions, MCP exposure. From
   `starter-flow*`, `starter-flow-node-loop` (or `-adk`),
   `starter-skills`.
3. **Six bundled rubix agent flows** — one per goal above. Loaded
   into the host's `FlowRegistry` at boot.
4. **Rubix domain tools** — Rust `impl Tool` for every action the
   agent flows dispatch. Loaded into the host's `ToolRegistry` at
   boot. Lives in `rubix-tools`.
5. **Rubix bundled skills** — `SKILL.md` files that gate the tools
   per goal. Loaded into the host's `SkillRegistry` at boot
   (approved by default — host-bundled). Lives in `rubix-skills`.
6. **Transports** — REST + SSE (axum), gRPC (tonic), MCP (stdio),
   CLI (clap). Wired through `starter-server` + `starter-grpc` +
   `starter-mcp` + `starter-cli`.
7. **OpenAPI** — emitted from `rubix-spi` via `utoipa`. Single source
   of truth for every client.
8. **Extensions** — third-party tools / skills / flows / node kinds
   contributed through `starter-ext-flow`. **Host code lives in
   starter, not rubix.**
9. **SDUI backend** — server-driven UI route resolver. From
   `starter-sdui-routes`; dashboard pages produced by rubix tools.
10. **Storage** — Postgres for state (`starter-store-postgres`),
    ClickHouse for history (`starter-store-clickhouse` +
    `starter-warehouse`).

That is the whole product. Anything not on that list is out.

## Hard rules (load-bearing)

There are **thirteen** rules, R1–R13. The section headers below are
authoritative — when a design doc or comment references "R*n*", it
means the rule with that header here. Renumbering requires a SCOPE
bump and a sweep of every design doc that cites a rule. Rules
inherited from starter are referenced **by name**, not by starter's
number (which can change upstream).

### R1 — One responsibility per file. 400 lines. Always.

| Limit | Value |
|---|---|
| Max lines per file | **400** |
| Max lines per function | **50** |
| Max public items per module | **~10** |
| Max nesting depth | **4** |

Name files after the concept (`disk_usage.rs`, not `utils.rs`). Split
before 400, not after.

### R2 — Upstream first. Rubix-specific stays in rubix; reusable goes to starter.

`rubix` is a **consumer** of `starter`. Never the reverse. Before
adding anything to a rubix crate, ask: *would any other starter
consumer benefit from this?* If yes → fix it in starter, bump, and
consume. If no → it lives in rubix.

**Committed upstream work** (tracked in
[`docs/design/STARTER-CHANGES.md`](docs/design/STARTER-CHANGES.md);
each item gates a phase):

| Upstream item | Blocks rubix phase | Why upstream |
|---|---|---|
| `starter-flow-node-loop` (ai-agent body) | Phase 1 | Every starter consumer needs the agent loop |
| `starter-skills` (SKILL.md + quarantine) | Phase 1 | Skills are useful without rubix |
| MCP prompts + resources surfaces in `starter-mcp` | Phase 1 (R12) | Every MCP consumer needs UX-grade surface |
| Typed agent event taxonomy in `starter-flow` | Phase 1 (R13) | SSE shape is consumer-agnostic |
| Recorded-LLM-response harness in `starter-server::testing` (or new `starter-ai-record`) | Phase 1 (R10) | Every consumer testing an agent needs this; live-LLM-per-PR is unaffordable |
| `starter-tool-sysdiag` (disk / db-size / flow-errors) | Phase 1 | Any starter consumer with an operator wants these |
| `starter-tool-sdui` (page-builder primitives) | Phase 3 | Matches existing `starter-tool-*` pattern |
| `starter-tool-flow-ops` (deploy / validate / lint / list) | Phase 3 | Flow ops are needed by every flow-using consumer |
| `starter-tool-clickhouse` (rule.write / mart.create / retention.set) | Phase 4 | Any ClickHouse consumer benefits |
| `cron-schedule` node kind in `starter-flow-nodes` | Phase 4 | Every consumer wants cron triggers |
| `clickhouse-query` node kind in `starter-flow-nodes` | Phase 4 | Reusable for any CH consumer |
| `starter-ext-flow` adapter | Phase 5 | Required by starter's agent SCOPE |

The `starter-tool-*` line is load-bearing: rubix's "tools" are
mostly *reusable* by R2's own test. The default assumption is
**upstream**; rubix-only is the exception, justified per tool.
Tools that genuinely stay rubix-side (`user.admin` is the clearest
candidate, because it consumes rubix's tenant model) live in
`rubix-tools`; everything else ships as `starter-tool-*` and rubix
re-uses.

**Phase exit checklist:** every phase ends with
1. The list of upstream PRs filed (merged or in review) appended to
   that phase's section in `STARTER-CHANGES.md`.
2. **One named starter capability that is better** — more generic,
   better-documented, easier to consume — because of rubix's work
   this phase. "Better" is shown, not asserted: link the before/
   after, name a new starter consumer who could use it.

A phase with zero PRs filed *or* no named improvement is a smell —
re-check that no rubix code is doing something that should be
reusable, and that the upstreaming wasn't just "shove rubix-shaped
code into starter."

**Process.** Upstream PRs follow starter's normal review process.
The rubix author does **not** self-merge into starter; a starter
maintainer reviews and merges. This is what keeps R2 honest under
schedule pressure.

**No starter forks.** If a capability is missing from starter, the
fix is *in starter*, not a parallel rubix crate. CI enforces this
via the "No starter fork" smoke test.

### R3 — Observable state is a node. Ephemeral state isn't.

State another subsystem, flow, MCP, or operator would want to read,
history, or react to belongs in the graph as a slot. Request-scoped
locks, caches, pools, builders — not nodes. The load-bearing test:
*would anyone outside this subsystem benefit from reading this?* Yes
→ node. No → in-memory field.

**Carve-out: identity is not a slot.** Sessions, tokens, OAuth flow
state, tenant + team membership, and authz decisions are owned by
`starter-auth-users` / `starter-auth-oauth` / `starter-authz` and live
in *their* tables, not the slot store. The slot store is gated by
authz; making it the home of identity would be circular. Identity is
exposed *read-only* via system kinds (e.g. `sys.identity.session`,
`sys.identity.tenant`) whose slot reads delegate to the auth crates.
Writes go through the auth crates' own APIs. See
`docs/design/AUTH.md`.

**Second carve-out: flow run state is owned by `starter-flow`.**
`SessionStore`, `RunStore`, the run checkpoint blobs — all live in
the flow engine. Rubix does not maintain a parallel store of run
state.

### R4 — The graph is the world (modulo R3's carve-outs)

For everything that *is* a slot, there is **one** API. Engine, flows,
MCP, CLI, REST, gRPC, extensions all read and write through the same
slot API on `starter-flow`. No parallel models, no shadow tables, no
"just this once" REST endpoints that mutate persistent state outside
the graph. The R3 carve-outs (identity, flow run state) are the only
documented exceptions; adding a third requires a SCOPE change.

### R5 — Layer arrow: `contracts → tools → flows → binary`

`rubix-spi` (contracts) depends only on `starter-spi`. `rubix-tools`
depends on `rubix-spi` + starter libraries. `rubix-skills` and
`rubix-flows` are content crates (markdown + YAML resources, no Rust
logic beyond load helpers). `rubix-agent` (the binary) depends on
everything and wires the transports.

**A tool file contains dispatch logic only.** The tool's REST DTOs,
proto messages, and MCP descriptor (purpose / when / when-not /
example / siblings — see R12) live in `rubix-spi`. The tool's
event-emission shim uses the typed taxonomy from `starter-flow`
(see R13). This split keeps tool files under R1's 400-line ceiling
without exception.

**Twenty-line handler ceiling.** REST/gRPC/CLI/MCP handlers do four
things: extract inputs, call a domain function or dispatch a tool,
shape the result, return. Smoke test: *if I swap REST for gRPC
tomorrow, how much of this file changes?* Answer must be "route
wiring + DTO shaping only."

### R6 — `rubix-spi` is the contracts hub. Zero internal deps.

Wire types only: REST DTOs decorated with `utoipa::ToSchema`, gRPC
proto, MCP tool descriptors (where rubix introduces new ones beyond
what starter already provides). Zero runtime logic, zero HTTP, zero
SQL. Depends on `starter-spi` (re-uses `Id<T>`, `Error`, `Page<T>`,
`Cursor`, `Principal`, `Authenticator`, `PolicyEngine`,
`ResourceRegistry`, `AiRunner`, `SecretStore`, `Quantity`, `Unit`,
`UnitSystem`, `ResolvedPreferences`, `MessageKey`, `Diagnostic`,
`Tool`) and nothing else internal.

**Per-user units, time format, theme.** REST DTOs carrying physical
quantities use `starter-spi`'s `Quantity` (canonical unit +
magnitude), not raw floats. The transport layer converts to user
prefs on read via `starter-prefs`; storage is always canonical.
Locking this in Phase 1 keeps i18n / unit handling out of every
later handler.

**Prefs reach the agent, not just the transport.** The `ai-agent`
node receives the caller's `ResolvedPreferences` via a
`sys.identity.preferences` system kind (R3 carve-out). Tool
implementations format natural-language replies through
`starter-prefs` + `starter-i18n` — never via raw strings. A tool
that returns "65°C" instead of using `Quantity::format(prefs)` is a
bug. Tool descriptors (see R12) state when a tool produces
user-facing text.

### R7 — Tools, skills, flows, node kinds all extend the same registries

Rubix ships defaults; extensions and operators add more — into the
**same** registries. There is no rubix-only registry parallel to
starter's.

| Registry | Rubix bundles | Extensions contribute via | Operators add via |
|---|---|---|---|
| `ToolRegistry` | `rubix-tools` | `block.yaml` `contributes.tools` | (rare; usually via extension) |
| `SkillRegistry` | `rubix-skills` (`SKILL.md` files; approved) | `block.yaml` `contributes.skills` (quarantined) | `$XDG_DATA_HOME/rubix/skills/` (approved) |
| `FlowRegistry` | `rubix-flows` (YAML files) | `block.yaml` `contributes.flows` | `$XDG_DATA_HOME/rubix/flows/` |
| `NodeKindRegistry` | starter built-ins | `block.yaml` `contributes.nodes` | (rare; usually upstream to starter) |

The `ai-agent` node never knows or cares which bucket a tool, skill,
flow, or node kind came from. That's the leverage.

**Skill-deny behavior is visible refusal, never silent drop.** When
the agent attempts a tool not in the active skill's `allowed_tools`
intersection (see starter's skill scope rule 4):
1. The dispatch fails fast with a typed error
   (`ToolError::SkillForbidden { tool, skill }`).
2. An `agent.tool.error` event (R13) fires carrying the skill id and
   the denied tool id.
3. The agent receives a tool result that is a localized
   `MessageKey` (`rubix.skill.denied`) explaining which skill is
   active and which tools it permits — so the next turn can
   self-correct.
4. **No auto skill-swap.** Changing the active skill mid-run would
   undermine R4 of starter's skill scope rules (selection happens
   once per outer flow run). If a different skill is needed, the
   flow author wires a `skill_hint` on a downstream node.

See `docs/design/SKILLS.md` for the full behavior + observability
contract.

### R8 — `AiRunner` is the only LLM seam (inherited from starter)

Inherited from starter — see the **"AiRunner is the only LLM seam"**
rule in [DOCS/agent/SCOPE.md](../DOCS/agent/SCOPE.md). Re-stated here
by name (not number) so renumbering upstream doesn't rot this
reference. Applies transitively to every rubix tool and flow that
touches an LLM. CI enforcement is upstream in
`starter-flow-node-loop` (or `-adk`).

### R9 — Versioning is add-only within a major

Rubix contract surfaces (`rubix-spi`, public client API, rubix
`block.yaml` contributions, rubix MCP tool descriptors, the six
bundled flows' input/output DTOs) are add-only within a major.
Breaking changes bump the major on the crate, the client, and the
binary together.

### R10 — Tests live with the code

Same PR, same diff.

- **Unit tests** live inline as `#[cfg(test)] mod tests` at the
  bottom of the file they cover. Pure functions, no I/O.
- **Integration tests** live in the crate's `tests/` directory and
  mirror `src/` one-to-one — find the integration test for
  `disk_usage.rs` at `tests/disk_usage_test.rs`. Use
  `starter-server::testing`, never hand-rolled servers. Database
  tests use the testcontainers pattern from `starter-store-postgres`.
- **Agent loop tests use a recorded-LLM harness.** Each rubix tool
  ships at least one round-trip test through the `ai-agent` node
  loop using recorded provider responses — no live LLM call in CI.
  The harness lives in starter (`starter-server::testing` or a new
  `starter-ai-record` crate per `STARTER-CHANGES.md`); rubix
  consumes it. Recording new fixtures is a manual step gated on the
  author, not CI. Live-LLM tests exist but run on a nightly
  schedule, not per PR.
- Each goal ships at least one bundled-flow round-trip test
  end-to-end.

### R11 — Comments explain *why*, never *what*

Doc-comments on every public item explaining purpose, defaults, edge
cases. No `// STAGE-1 done`, no `// FIXED:`, no emoji banners, no
`// Previously this used Y`. TODOs carry a name or ticket.

### R12 — MCP surface = tools + prompts + resources, all three

Tools alone is "callable, not usable." A great MCP UX has three
surfaces, and rubix ships all three for every one of the six goals:

- **Tools** — what an LLM can *do*. Every bundled flow is a tool via
  `FlowAsTool`.
- **Prompts** — discoverable starting points. Every bundled flow
  ships a matching MCP prompt ("Build me a dashboard for…", "Show
  failing flows from the last 24h") so a Claude Desktop user picks a
  prompt, not a tool name.
- **Resources** — readable state for cheap grounding. Every goal
  exposes at least one resource (current dashboard list, flow
  registry, recent system-check results, weekly report archive).
  R3/R4 make this almost free — most resources are slot reads.

**Tool descriptors are UX, not metadata.** Every rubix tool
descriptor in `rubix-spi` includes:
1. **Purpose** — one sentence, plain.
2. **When to use** — concrete trigger conditions.
3. **When NOT to use** — the most common misuse.
4. **Example** — one input + one output, realistic, ≤10 lines.
5. **Siblings** — names the tool(s) most likely to be confused with
   this one, and one phrase per sibling explaining when *this* tool
   wins. With ~25 rubix tools across six goals, disambiguation is
   where the LLM lives or dies. Empty siblings field allowed *only*
   if the tool truly has no near-neighbours (rare).

Descriptor authoring is the single highest-leverage UX lever in this
backend. Descriptors steer **both** the bundled rubix agent and
external MCP clients — the bar is the same. See
`docs/design/MCP-UX.md`. Empty or one-line descriptors fail review.

**Descriptor calibration test (Phase 1 exit gate).** For each goal,
two reviewers are independently given (a) a realistic user prompt
and (b) the descriptors of every tool in that goal — no SCOPE, no
flow YAML, no context. Each picks the tool the agent should call.
**Reviewers must agree on at least 80% of prompts.** Disagreement
points at descriptors that fail to disambiguate; fix them, re-run.
This makes "good descriptor" measurable, not present/absent.

**Resource URI scheme is locked.** Every MCP resource URI rubix or
its extensions expose has the shape `rubix://<goal>/<resource>` —
e.g. `rubix://system/last-check`, `rubix://dashboard/pages`,
`rubix://flow/registry`. Extensions follow the same scheme using
their `block.yaml` id as the `<goal>` segment
(`rubix://com.acme.foo/<resource>`). Fragmenting the namespace
(`mycorp://...`) fails review.

**Starter dependency:** MCP prompts + resources surfaces must exist
in `starter-mcp`. If they don't yet, that's an upstream PR per R2 —
tracked in `docs/design/STARTER-CHANGES.md`.

### R13 — Agent observability is a first-class SSE surface

For a backend whose primary surface is "agent doing stuff," SSE
events **are** the UX. There is one typed event taxonomy, shared
across every transport that streams (REST/SSE, gRPC server-stream,
MCP `notifications/progress`). Tool authors do not invent event
shapes.

The taxonomy (lives in `starter-flow` event types per R2; rubix
consumes — see `docs/design/STARTER-CHANGES.md` if missing):

| Event | Emitted by | Carries |
|---|---|---|
| `agent.turn.start` | `ai-agent` node | turn id, skill id |
| `agent.thinking` | `ai-agent` node | partial token stream |
| `agent.tool.start` | `ai-agent` node | tool id, args |
| `agent.tool.complete` | `ai-agent` node | tool id, result, duration |
| `agent.tool.error` | `ai-agent` node | tool id, error |
| `flow.step` | engine | node id, slot writes |
| `slot.write` | engine | path, before, after |
| `skill.match` | `SkillSelector` | skill id, score |
| `progress` | long-running tool | percent + message |

**Long-running tools (>2s) emit `progress` at least once every 5
seconds.** Otherwise the user sees a hang. The agent's *between-
tool* thinking phase is just as bad as a tool hang from the user's
perspective.

**MCP mapping is pinned, not optional.** The MCP transport in
`rubix-agent` maps:
- `agent.thinking` (token stream) → `notifications/progress`
  with `progress.message` carrying the partial text. Claude
  Desktop renders continuous motion.
- `agent.tool.start` → `notifications/progress` with
  `progress.message` = "calling <tool>".
- `agent.tool.complete` / `agent.tool.error` → progress updates
  + the tool result on the response.
- `progress` events from long tools → `notifications/progress`
  with their percent + message verbatim.

If the upstream MCP prompts/resources work (see
`STARTER-CHANGES.md`) lands a richer progress shape, rubix adopts
it; the mapping above is the floor.

**Cancellation UX.** A canceled flow surfaces a clean, localized
message — `MessageKey::new("rubix.flow.canceled")` rendered through
`starter-i18n` — **not** a stack-trace-shaped error. The MCP client
sees a normal tool response carrying the message, plus an
`agent.tool.error` event with `reason: canceled`. Stack traces in
user-facing cancel paths fail review. The `Cancel` token (starter's
existing mechanism) is the trigger; tools observe it between steps.

**Strings in events are `MessageKey`, not raw text.** The transport
resolves to the caller's locale via `starter-i18n`. A tool emitting
`"Disk full"` instead of `MessageKey::new("rubix.system.disk_full")`
is a bug.

**Skill observability.** Every agent turn records which skill was
active (`agent.turn.start` carries it). Querying "why did the agent
do that?" → grep traces by turn id, see the skill, read the skill
body. See `docs/design/EVENTS.md` for the full event schema.

## Repo layout

```
rubix/
  SCOPE.md
  Cargo.toml                      <- workspace
  mani.yaml                       <- task orchestrator

  crates/
    rubix-spi                     <- R6. Contracts only. Re-exports
                                     starter-spi. REST DTOs for rubix
                                     tools (utoipa), proto where rubix
                                     introduces new gRPC services.

    rubix-tools                   <- THE CORE OF THE PRODUCT.
                                     impl starter_spi::Tool for every
                                     rubix-specific action the agent
                                     dispatches. One file per tool,
                                     ≤400 lines (R1). Grouped into
                                     modules per goal:
                                       dashboard/, user/, flow/,
                                       clickhouse/, system/, analytics/

    rubix-skills                  <- SKILL.md bundles, one per goal.
                                     Almost no Rust — include_dir!
                                     macro + load helper. The content
                                     is markdown.

    rubix-flows                   <- flow YAML bundles, one per goal.
                                     Almost no Rust — include_dir! +
                                     load helper. The content is YAML.

    rubix-client                  <- Thin extension of
                                     starter-client-rs. Adds rubix
                                     endpoints (the rubix tools'
                                     REST surface). Used by CLI,
                                     tests, third-party Rust consumers.

    rubix-agent                   <- THE BINARY. main.rs (~100 lines)
                                     wires:
                                       FileSecretStore
                                       starter-ai Claude CLI runner
                                       ToolRegistry  ← rubix-tools +
                                                       ext-contributed
                                       SkillRegistry ← rubix-skills +
                                                       host dir +
                                                       ext-contributed
                                       FlowRegistry  ← rubix-flows +
                                                       host dir +
                                                       ext-contributed
                                       NodeKindRegistry ← starter
                                                          built-ins +
                                                          ext-contributed
                                       starter-flow Engine
                                       starter-ext-host
                                       starter-mcp router (automatic)
                                       starter-server (REST + SSE)
                                       starter-grpc
                                       starter-cli
                                       starter-auth-users + oauth
                                       starter-authz
                                       starter-store-postgres
                                       starter-store-clickhouse +
                                         warehouse

  extensions/
    com.rubix.example/            <- reference block contributing
                                     extra tools/skills/flows. Layout
                                     template for third-party authors.

  docs/
    design/
      OVERVIEW.md                 <- repo map + dep arrow
      AGENT.md                    <- how the six goals map onto
                                     ai-agent + tools + skills +
                                     flows. References starter's
                                     DOCS/agent/SCOPE.md.
      TOOLS.md                    <- how to add a rubix tool +
                                     descriptor authoring contract
                                     (R12)
      SKILLS.md                   <- how to author a rubix skill;
                                     the bundled six; hot-reload
                                     semantics; skill observability
      FLOWS.md                    <- how the bundled flows compose
                                     tools and the ai-agent
      MCP-UX.md                   <- R12 in full: prompts +
                                     resources + descriptor template
                                     + Claude Desktop smoke walk
      EVENTS.md                   <- R13 in full: SSE event schema,
                                     MessageKey usage, progress
                                     cadence
      AUTH.md                     <- session, JWT, OAuth, authz
                                     wiring + the R3 identity carve-out
      MIGRATIONS.md               <- starter + rubix migration order
      EXTENSIONS.md               <- block author guide + 10-minute
                                     scaffold walkthrough
      SDUI.md                     <- server-driven UI surface +
                                     per-user dynamic resource authz
      WAREHOUSE.md                <- ClickHouse marts, L1→L3 rules
      STARTER-CHANGES.md          <- THE upstream PR list. Every
                                     starter capability rubix needs
                                     that doesn't yet exist, ordered
                                     by which phase blocks on it.
                                     This is how R2 becomes a
                                     deliverable, not a slogan.
      TESTS.md                    <- R10 in full
      VERSIONING.md               <- R9 in full
    sessions/                     <- working notes, NOT design
```

Six Rust crates. One extension example. Fifteen design docs.
**Phase 0 exit gate: every design doc above exists as at least a
one-page stub citing the SCOPE rule(s) it expands on.** Dead
cross-refs fail review.

The rubix docs reference starter's authoritative format docs (the
`SKILL.md` schema lives in starter's `DOCS/agent/SKILLS.md`; the
flow YAML schema lives in starter's flow SCOPE). Rubix docs cover
*how rubix bundles its six skills/flows* and rubix-specific
conventions, not the format itself.

## Dependency arrow (Rust)

```
starter-spi
   ↑
rubix-spi
   ↑
   ├── rubix-client                (HTTP client; zero agent dep)
   ├── rubix-tools                 (impl Tool for the rubix actions)
   ├── rubix-skills                (SKILL.md bundles via include_dir)
   ├── rubix-flows                 (flow YAML bundles via include_dir)
   │       │
   │       └──── all four consumed by ────┐
   │                                      │
   │                                      ▼
   │                                rubix-agent  (the binary)
   │
   └── starter-*  (via cargo features the binary chooses)
                  starter-flow, starter-flow-node-loop, starter-skills,
                  starter-ext-flow, starter-server, starter-grpc,
                  starter-mcp, starter-cli, starter-auth-users,
                  starter-auth-oauth, starter-authz, starter-ai,
                  starter-store-postgres, starter-store-clickhouse,
                  starter-warehouse, starter-sdui-routes, starter-i18n,
                  starter-prefs, starter-observability, starter-secrets-file,
                  starter-jsonrpc-stdio, starter-config, starter-tags,
                  starter-audit, starter-insights
```

Never the other way: no tool/skill/flow crate consumes the agent
binary; `rubix-spi` never depends on anything internal; nothing in
rubix forks a starter capability.

## Starter crates rubix consumes

| Capability | Starter crate(s) |
|---|---|
| Contracts | `starter-spi` |
| Config | `starter-config` |
| Logging / tracing / metrics | `starter-observability` |
| HTTP server, OpenAPI, SSE, middleware | `starter-server` |
| Auth (users + sessions + tokens) | `starter-auth-users` |
| OAuth (GitHub, Google) | `starter-auth-oauth` |
| AuthZ (tenants, teams, audit) | `starter-authz` |
| Secrets | `starter-secrets-file` (default), `-keyring` (opt-in) |
| Postgres | `starter-store-postgres` |
| ClickHouse + warehouse | `starter-store-clickhouse`, `starter-warehouse` |
| Flow engine | `starter-flow`, `starter-flow-spi`, `starter-flow-nodes` |
| Flow hot-reload | `starter-flow-watch` |
| `ai-agent` node kind (LLM loop) | `starter-flow-node-loop` **(planned upstream — see [STARTER-CHANGES.md](docs/design/STARTER-CHANGES.md))** |
| Skill format + registry + quarantine | `starter-skills` **(planned upstream)** |
| Extension host (supervisor, spi, sdk, mcp, grpc, wasm) | `starter-extensions/crates/starter-ext-*` (sibling workspace, exists) |
| Extension flow/skill/tool adapter | `starter-ext-flow` **(planned upstream — referenced by starter agent SCOPE; not yet built)** |
| MCP | `starter-mcp` |
| gRPC | `starter-grpc` |
| CLI building blocks | `starter-cli` |
| HTTP client (extended by `rubix-client`) | `starter-client-rs` |
| SDUI route resolver | `starter-sdui-routes` |
| i18n | `starter-i18n` |
| User prefs (units, time, theme) | `starter-prefs` |
| AI provider seam (Claude CLI, etc.) | `starter-ai` |
| Tagging | `starter-tags` |
| Audit log | `starter-audit` |
| Insights | `starter-insights` |
| Extension JSON-RPC channel | `starter-jsonrpc-stdio` |

If a capability is missing, the fix is *in starter*, not a parallel
rubix crate.

**Deliberately not used:** `starter-auth-token` (single-owner bearer
for headless appliances) — rubix is multi-user from day one.
`starter-store-sqlite` — Postgres only (Non-goals). `starter-tauri`
— not used in this tree. The `starter-ui-*` packages are consumed
by [`rubix/frontend/`](frontend/) and the planned
[`rubix/mobile/`](docs/scope/mobile/) per
[ADR 0004](docs/adr/0004-react-native-mobile-app.md); the
backend crates themselves do not depend on them.

## Where does my code go? — short decision tree

1. **Wire type?** (REST DTO, proto message, MCP tool descriptor) →
   `rubix-spi`. Then regenerate OpenAPI / proto.
2. **Rubix action the agent should be able to take?** → `impl Tool`
   in `rubix-tools`, in the matching goal module
   (`dashboard/`, `user/`, `flow/`, `clickhouse/`, `system/`,
   `analytics/`). One file per tool, ≤ 400 lines.
3. **Steering / policy for one of the six goals?** → a `SKILL.md`
   in `rubix-skills` under that goal's directory.
4. **Composition of tools into a goal flow?** → a flow YAML in
   `rubix-flows`. Root node is `ai-agent`.
5. **REST / gRPC / MCP / CLI handler?** → matching module in
   `rubix-agent`. 20-line ceiling. Domain action stays in
   `rubix-tools`. MCP exposure is automatic for flows — *don't*
   write per-flow MCP handlers.
6. **Migration?** → `rubix-agent/migrations/`, run after starter
   migrations. See `docs/design/MIGRATIONS.md`.
7. **Out-of-process block?** → `extensions/com.<org>.<name>/` per
   starter's `block.yaml` format. **Host code stays in starter**
   (`starter-ext-flow`).
8. **Capability that any other starter consumer would want?** → fix
   it *in starter* and bump (R2). Likely upstream targets:
   `starter-flow-nodes` for reusable node kinds; new `starter-tool-*`
   crates for reusable tools.
9. **Doc?** → `docs/design/` (architecture), `docs/sessions/`
   (working notes).
10. **Unsure?** → read `docs/design/OVERVIEW.md` and starter's
    `DOCS/agent/SCOPE.md`, then ask. One sentence beats two hours of
    refactoring the wrong direction.

## Smoke tests (before merging anything)

### "Six goals reach MCP automatically" test

A Claude Desktop client connects to the rubix MCP endpoint. It sees
exactly the six bundled flows as tools:
`com.rubix.dashboard-assistant`, `com.rubix.user-admin`,
`com.rubix.flow-programmer`, `com.rubix.clickhouse-ruler`,
`com.rubix.scheduled-system-check`, `com.rubix.weekly-report`
(plus any extension-contributed flows). Calling any one of them runs
the flow server-side; tokens stream via `notifications/progress`.
**No rubix code wires MCP per flow.** If any flow needed a bespoke
MCP handler, `FlowAsTool` slipped or rubix re-implemented it.

### "MCP UX is three surfaces" test (R12)

The same MCP client sees **six prompts** (one per goal) and at
least **six resources** (one per goal). A human picks a prompt,
never types a tool name, and reaches the end of a goal task. Every
tool descriptor has all four fields (purpose / when / when-not /
example). A reviewer can read a descriptor cold and predict when
the agent will pick that tool. Missing fields → fail.

### "SSE event taxonomy is honoured" test (R13)

Run any bundled flow with SSE attached. The stream contains
`agent.turn.start`, `agent.thinking`, `agent.tool.start`,
`agent.tool.complete`, `flow.step`, `slot.write` events in plausible
order. No raw strings (every text field is a `MessageKey` that
resolves through `starter-i18n`). A 10-second tool emits at least
one `progress` event. A `Cancel` token fires → the next event is
`agent.tool.error` with cancellation reason, then no further events
for that turn.

### "Starter improvements per phase" test (R2)

At each phase exit, `docs/design/STARTER-CHANGES.md` has a populated
section for that phase listing at least one upstream PR (merged, in
review, or filed-with-rationale). A phase with zero is a smell:
either you didn't try to upstream (R2 slipped) or rubix is doing
something that should be reusable. The phase exit reviewer asks
"what didn't get upstreamed and why?" and the answer is on the page.

### "Upstream first" test

Walk every public item in `rubix-tools`. For each, ask: *would any
other starter consumer benefit?* If yes and the item isn't already
in starter — open an issue to upstream. R2 is a process rule, not
just a slogan.

### "No starter fork" test

Grep `rubix/crates/` for re-implementations of starter capabilities
(HTTP server bootstrap, store wrappers, auth middleware, OpenAPI
emission, agent loop, skill parser, tool registry, extension host).
Any hit → R2 slipped. Fix in starter, bump, delete the fork.

### "Swap REST for gRPC" test

Pick a handler in `rubix-agent`. Swapping the transport would require
rewriting only route wiring + DTO shaping. Anything else means a tool
or domain action leaked into transport — move it to `rubix-tools`.

### "Build a new client" test

If someone deletes `rubix-client` and only has `rubix-spi`'s OpenAPI
snapshot + `starter-client-rs` from crates.io, can they generate a
working client? If not, `rubix-spi` is missing a `ToSchema` or a DTO.

### "Build a new block" test

If someone has only starter's `block.yaml` format + `rubix-spi`
(for any rubix tool DTOs they want to call), can they build and ship
an extension that contributes a tool, skill, or flow? If they need
to path-dep `rubix-tools` / `rubix-flows` / `rubix-agent`, the
extension surface is wrong — fix it (probably by upstreaming the
needed type to starter or `rubix-spi`).

### "Postgres-only" test

`cargo tree` from `rubix-agent` shows zero resolution to
`starter-store-sqlite` or any `*-sqlite-*` crate. Rubix is Postgres
only — rubix decision (starter ships both; rubix picks Postgres for
warehouse + migration parity).

### "Observable state is a node" test

Walk every `Mutex<…>` / `RwLock<…>` in `rubix-tools`. For each, ask:
*does anything outside this subsystem need to read this?* If yes and
no slot exposes it — R3 slipped.

### "AI loads context cleanly" test

Pick any file in `rubix/`. Under 400 lines? Name describes a single
concept? Test at `tests/<same_name>_test.rs`? Any "no" → R1 slipped.

## Non-goals (this scope)

- **Frontend surfaces live in their own trees.** [`rubix/frontend/`](frontend/)
  is the web SPA; [`rubix/mobile/`](docs/scope/mobile/) is the
  planned React Native app. Both consume the backend; the backend
  crates do not depend on either. See
  [ADR 0004](docs/adr/0004-react-native-mobile-app.md).
- **No second agent runtime.** The `ai-agent` node kind from starter
  is the agent. No `Agent` trait, no rubix LLM loop.
- **No second tool/skill/flow registry.** Same registries as starter.
- **No extension host code in rubix.** `starter-ext-flow` already
  handles `contributes.{tools,skills,flows,nodes}`.
- **No Zenoh / fleet.** Single-agent v0. Fleet transport is a later
  scope built on `starter-flow-surfaces` when the need is real.
- **No SQLite.** Postgres only — rubix decision.
- **No SSO provider.** Rubix consumes `starter-auth-users` +
  `starter-auth-oauth`. OIDC consumers swap the `Authenticator`
  trait.
- **No parallel storage layer.** `starter-store-postgres` +
  `starter-store-clickhouse` are the only entry points.
- **No domain-specific verticals.** Devices, drivers, schedules,
  alarms, histories are *not* in v0. The six goals above are the
  scope. Verticals can ship later as extensions or as their own
  product on top of rubix.
- **No backup/restore in v0.** Six places hold state (Postgres,
  ClickHouse, slot store, skills dir, flows dir, extension state).
  An operator-grade backup story is a later scope, not Phase 0–5.
- **No agent rate limiting / per-tenant token quota in v0.** An
  MCP client can call the agent in a loop; token cost is real
  money. A Phase 6+ concern; flagged here so reviewers don't
  assume it's covered.

## Decisions made (locked)

- **Backend product first.** Frontend surfaces live in their own
  trees ([`rubix/frontend/`](frontend/) today,
  [`rubix/mobile/`](docs/scope/mobile/) planned) and consume the
  backend per [ADR 0004](docs/adr/0004-react-native-mobile-app.md).
- **One repo, two trees.** `rubix/` sibling to `starter/crates/`.
  Path-deps for now; registry deps when starter stabilises.
- **Postgres only** for state. ClickHouse for history.
- **Rust only.** No Go, no Dart, no TS in this tree.
- **AuthZ via `starter-authz` Phase 7.** Tenants + teams + audit.
  No parallel RBAC.
- **The agent is starter's `ai-agent` node kind**, dispatching tools
  from a shared `ToolRegistry`, steered by skills from a shared
  `SkillRegistry`, composed into flows in a shared `FlowRegistry`.
  Rubix ships defaults into all three.
- **Upstream first (R2).** Reusable capability → starter. Rubix-only
  → rubix.
- **`mani` is the workspace task orchestrator.** `mani.yaml` lists
  build/test/status tasks; `mani run build --all` is the canonical
  pre-commit check.

## Provisional choices (revisit when warranted)

- **Six rubix crates at Phase 0.** `rubix-spi`, `rubix-tools`,
  `rubix-skills`, `rubix-flows`, `rubix-client`, `rubix-agent`.
  R1 (400 lines) takes precedence — if `rubix-tools` outgrows
  itself, split per goal (`rubix-tools-dashboard`,
  `rubix-tools-user`, etc.).
- **Default extension transport: stdio + JSON-RPC** via
  `starter-jsonrpc-stdio` (matches starter's extension framework).

## Thin-slice plan

The five phases below remain the long-term shape. The **active
working plan** is [`docs/scope/THIN-SLICE.md`](docs/scope/THIN-SLICE.md):
one demo path that exercises every architectural layer
end-to-end (auth, authz, audit, flow runtime, agent, MCP, tools,
prefs, i18n, Postgres, ClickHouse, insights, CLI), shipped in
five PRs.

After the thin slice lands, each phase below becomes "broaden an
already-working layer" rather than "add a layer that didn't
exist." This is more valuable than depth in one phase because
every later change has a working end-to-end test to validate
against.

## Forward-looking gaps

[`docs/scope/GAPS.md`](docs/scope/GAPS.md) is the rolling audit of
starter capabilities this SCOPE has not yet accounted for —
specifically undo/redo, clipboard, audit log, insights, blob
storage, export, tags, alert sinks, `FlowAsService` naming,
config layering, and the **critical i18n + user-preferences
end-to-end demonstration**. Read GAPS.md at every phase entry; the
"Summary table" maps each gap to the phase that should absorb it.

A gap promoted out of GAPS.md becomes a SCOPE rule update, a new
phase deliverable, or a Non-goal — never a "we'll do it later"
without a tracked home.

## Open questions

Each question is owned by a phase entry gate — resolve before that
phase starts.

| # | Question | Resolve in | Gate |
|---|---|---|---|
| Q1 | `starter-flow-node-loop` vs `-adk` (starter D1) | starter's `DOCS/agent/SCOPE.md` D1 | Phase 1 entry |
| Q2 | Migration ordering across starter + rubix | `MIGRATIONS.md` | Phase 2a entry |
| Q3 | Where dashboard SDUI pages physically live (`starter-sdui-routes` store vs rubix Postgres table) | `SDUI.md` | Phase 3 entry |
| Q4 | Cron via `starter-flow` Service surface or new node kind | `STARTER-CHANGES.md` + verification | Phase 4 entry |
| Q5 | Multi-tenant ClickHouse isolation (per-tenant tables / per-row column / separate DBs) | `WAREHOUSE.md` | Phase 4 entry |
| Q6 | Does `rubix-client` need to exist as a crate, or can rubix endpoints be a `starter-client-rs` re-export module? | OVERVIEW.md | Phase 1 entry |

## Phases

Strictly ordered, each assumes the previous landed. Phases are
**driven by the six goals** — by the end of Phase 4, all six work
end-to-end via MCP. Phase 5 adds extension contribution on top.

### Phase 0 — Skeleton

- Create the six crates as empty workspace members.
- `rubix-agent` boots: `starter-server` with one `/healthz` route,
  `starter-observability` wired, Postgres connection from
  `starter-store-postgres`, an empty `Engine`, an empty
  `ToolRegistry`, an empty `SkillRegistry`, an empty `FlowRegistry`.
  No domain logic yet.
- `mani run build --all` green.
- Exit: `cargo run -p rubix-agent` serves `GET /healthz` and logs a
  structured startup line listing every registry's size (all 0).

### Phase 1 — First goal end-to-end (Goal 5: system check)

The smallest useful slice with **no auth dependency** (auth lands in
Phase 2a). System check tools touch only the local host and rubix's
own Postgres metadata.

**Entry gate — i18n + prefs end-to-end (from `docs/scope/GAPS.md` #1):**
- EN + ES MessageKey catalogues shipped from day one (a tiny
  `rubix-i18n` content crate or `rubix-spi::i18n` module, same
  `include_dir!` shape as `rubix-skills`).
- At least one `Quantity`-typed tool output (disk usage GB/GiB)
  that round-trips through `starter-prefs` and renders in the
  caller's units.
- Every bundled rubix SKILL.md has a "Localisation" section
  telling the agent to emit MessageKeys + Quantity values,
  never raw strings or floats.
- See `docs/design/AI-PROVIDERS.md` for which `starter-ai`
  provider Phase 1 selects and how (GAPS.md #19).

- `rubix-spi`: REST DTOs for `system.disk`, `system.db`,
  `system.flow-errors`. `ToSchema` everywhere. Tool descriptors per
  R12 (purpose / when / when-not / example).
- `rubix-tools`: `impl Tool` for the three system tools + a typed
  event emitter shim per R13.
- `rubix-skills`: `skills/system-checker/SKILL.md` with
  `allowed_tools: [system.*]`.
- `rubix-flows`: `flows/scheduled-system-check.yaml` — single
  `ai-agent` node, `trigger: explicit` (cron added Phase 4 once the
  upstream node kind exists), `session_policy: fresh`.
- `rubix-agent`: wire `starter-ai` (Claude CLI runner),
  `starter-flow-node-loop`, `starter-skills`, the registries, MCP
  router. SSE event stream wired per R13.
- One MCP prompt + one MCP resource shipped per R12
  ("Run a system check" prompt; `rubix://system/last-check` resource).
- OpenAPI emitted; `rubix-client` codegens from it.
- Exit:
  - Claude Desktop connects, sees the
    `com.rubix.scheduled-system-check` tool, the matching prompt,
    and the resource. Picking the prompt runs the flow; progress
    streams as `notifications/progress`; cancellation produces a
    clean localized message.
  - **Descriptor calibration test (R12) passes** at ≥80% reviewer
    agreement across the three system-check tools.
  - **Recorded-LLM harness in place** — no live-LLM CI calls.
- Upstream PRs filed: `starter-flow-node-loop`, `starter-skills`,
  MCP prompts+resources, event taxonomy, recorded-LLM harness,
  `starter-tool-sysdiag`.
- **Named starter improvement (R2):** one capability that's better
  because of this phase — likely the MCP UX work or the recorded-
  LLM harness. Linked in `STARTER-CHANGES.md`.

### Phase 2a — Auth + authz on REST + Goal 2 (user-admin)

Adding auth is the natural moment to ship the user-admin goal —
they share dependencies.

**Entry gates (must be resolved before code lands):**
- `MIGRATIONS.md` finalized — starter+rubix migration order
  documented and tested.
- `AUTH.md` finalized — the R3 identity carve-out and OAuth-via-MCP
  flow specified.
- `AUDIT.md` finalized — `starter-changelog` + `starter-audit`
  + `starter-agent-log` wiring; every user-admin write tool
  produces a changelog row; every `agent.turn.start` emits an
  agent-log row carrying the active skill (GAPS.md #4).
- `CONFIG.md` finalized — `starter-config` layered loader replaces
  the Phase 0 `RUBIX_BIND` env var (GAPS.md #15).

- `starter-auth-users` + `starter-auth-oauth` wired (sessions,
  tokens, OAuth callback).
- `starter-authz` gating every REST surface from Phase 1 + the new
  user-admin surface.
- R3 identity carve-out implemented: `sys.identity.session`,
  `sys.identity.tenant`, `sys.identity.team`,
  `sys.identity.preferences` system kinds expose auth + prefs state
  read-only via slot reads that delegate to the auth + prefs crates.
- **Goal 2 (user-admin)** tools, skill, flow, prompt, resource —
  same R12/R13 contract as Phase 1.
- `docs/design/AUTH.md` written and merged before any code lands.
- Exit: an unauthenticated request returns 401; an authenticated
  request with the wrong tenant returns 403; the rubix MCP endpoint
  requires auth; a Claude Desktop user authenticated as a tenant
  admin can ask user-admin to list users in their tenant.
- Upstream PRs filed: any auth/authz rough edges discovered while
  wiring (e.g. missing DTOs, awkward middleware composition).
- **Named starter improvement (R2):** one auth/authz capability
  that's better because of rubix's wiring (e.g. an MCP-transport
  Authenticator shape, or a tenant-scoped query helper).

### Phase 2b — gRPC + CLI on the same surface

- gRPC server (`starter-grpc`) exposes the same tool surface as
  REST, behind the same authz checks. (MCP is already wired in
  Phase 1.)
- CLI (`starter-cli` + `rubix-client`) hits the same endpoints.
- Exit: same `user.list` operation works from REST, gRPC, MCP, and
  CLI, all behind authz; SSE event taxonomy (R13) renders coherently
  on the gRPC server-stream too.
- Upstream PRs filed: any gRPC/CLI rough edges discovered.
- **Named starter improvement (R2):** one CLI or gRPC capability
  more reusable than before (e.g. "subcommand per Tool"
  auto-generation in `starter-cli`).

### Phase 3 — Goals 1 + 3 (dashboards + flow programmer)

**Entry gates (from `docs/scope/GAPS.md`):**
- `starter-undo` wired — every write tool registers a
  `Reversible`; `rubix.undo.last` tool bundled (GAPS #2).
- `starter-clipboard` wired — `dashboard.duplicate` and
  `flow.duplicate` delegate (GAPS #3).
- Blob backend decided (`starter-blob-fs` default,
  `starter-blob-s3` opt-in) — pages with embedded images use it
  (GAPS #7).
- `starter-tags` toolset added (`rubix.tag.{create,assign,list}`)
  or confirmed deferred to Phase 4 (GAPS #9).

- **Goal 1 (dashboard):** `dashboard.{create,update,list,page.set}`
  tools. `starter-sdui-routes` wired; pages produced by the
  `dashboard.page.set` tool are stored per `SDUI.md` resolution.
  `flows/dashboard-assistant.yaml` bundled.
- **Goal 3 (flow programmer):** `flow.{deploy,validate,lint,list}`
  tools. `starter-flow-watch` wired for hot reload of operator-
  dropped flows. `flows/flow-programmer.yaml` bundled.
- Each goal ships its MCP prompt + resource per R12.
- Exit: an MCP caller can ask the dashboard assistant to "make me a
  page showing X" and the flow programmer to "deploy this flow YAML
  and tell me if it lints"; both stream R13 events correctly.
- **Upstream PRs (committed):**
  - `starter-tool-sdui` (page-builder primitives) lands in starter
    before the dashboard tools consume it.
  - `starter-tool-flow-ops` (deploy/validate/lint/list) lands;
    rubix consumes it for the flow-programmer goal.
  - If review on either takes too long, the primitives stay in
    `rubix-tools::<goal>::*` *with a tracking issue to move them
    upstream*, never as a "we'll do it later."
- **Named starter improvement (R2):** SDUI page-builder primitives
  and/or flow-ops are now usable by any starter consumer with no
  rubix dependency.

### Phase 4 — Goals 4 + 6 (ClickHouse + analytics) + cron triggering

**Entry gates (from `docs/scope/GAPS.md`):**
- Multi-tenant ClickHouse isolation resolved in `WAREHOUSE.md`
  (per-tenant tables vs. per-row tenant column vs. separate
  databases). Tenancy is load-bearing; pick once.
- `starter-export` wired into the analytics flow; rendered
  reports land in the blob store from Phase 3 (GAPS #8).
- `starter-insights` wired for system-check thresholds and the
  weekly-report's "is this metric out of band?" logic (GAPS #5).
- `FlowAsService` (from `starter-flow-surfaces`) named explicitly
  as the cron triggering mechanism — not a rubix scheduler
  (GAPS #16).

- **Goal 4 (ClickHouse):** `clickhouse.{rule.write,mart.create,
  retention.set}` tools. `starter-store-clickhouse` +
  `starter-warehouse` wired. `flows/clickhouse-ruler.yaml` bundled.
- **Goal 6 (analytics):** `analytics.{query,report}` tools.
  `flows/weekly-report.yaml`.
- **Cron triggering for Goal 5:** Phase 1's system-check flow
  changes `trigger: explicit` → `trigger: schedule(cron = ...)`.
- Exit:
  - Scheduled system checks run every 15 minutes and alert on
    failure; an MCP caller can request a weekly analytics report.
  - **All six goals reach MCP end-to-end** — Phase 5 adds extension
    contribution, not the sixth goal.
- **Upstream PRs (committed):**
  - `cron-schedule` node kind in `starter-flow-nodes` lands before
    cron triggering goes live.
  - `starter-tool-clickhouse` lands; rubix consumes it.
  - `clickhouse-query` node kind in `starter-flow-nodes` if any
    rubix flow YAML calls ClickHouse directly.
- **Named starter improvement (R2):** the new ClickHouse tooling +
  warehouse marts pattern is now usable by any starter consumer
  with ClickHouse, not just rubix.

### Phase 5 — Extension contribution

- `extensions/com.rubix.example/` ships a `block.yaml` contributing
  one extra tool, one skill, and one flow.
- `starter-ext-flow` (planned upstream, see STARTER-CHANGES.md)
  loads it on rubix boot.
- Exit:
  - The example extension's tool / skill / flow appear in the
    shared registries; the agent uses them indistinguishably from
    bundled ones.
  - `block.yaml` `auth:` declarations are enforced by
    `starter-ext-flow` at the boundary.
  - **Ergonomic goal: a fresh extension author scaffolds a new
    tool/skill/flow in ≤10 minutes** following
    `docs/design/EXTENSIONS.md`. Measured by a real walkthrough;
    failures of this test block phase exit.
- **Upstream PRs (committed):** `starter-ext-flow` itself, **plus
  every ergonomic gap the 10-minute walkthrough surfaces, filed
  upstream *before* phase exit**, not after. The walkthrough is
  not done until every gap is either filed or fixed.
- **Named starter improvement (R2):** the extension-author surface
  is the strongest generic-framework test — name the single biggest
  improvement landed upstream.

## Bottom line

**`starter` is the platform. `rubix` is one product assembled from
starter parts plus a thin layer of tools, skills, and flows for six
concrete goals: dashboards, users, flows, ClickHouse rules,
background jobs, analytics.** Six rubix crates, one binary, every
transport (MCP automatic), no frontend. If a capability fits in
starter, it goes in starter — every other consumer wins, and rubix
stays small.
