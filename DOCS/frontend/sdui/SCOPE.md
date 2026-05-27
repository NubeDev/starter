# `sdui` — Scope

> ⚠ **Read these first.** This doc carves out the typed-UI substrate
> the rest of starter's frontend story sits on. If anything below
> contradicts them, they win.
>
> - [DOCS/flow/scope/SCOPE.md](../../flow/scope/SCOPE.md) — the runtime
>   substrate. Pages that bind to live data subscribe through the same
>   slot-write propagation the flow engine owns.
> - [DOCS/frontend/theme/README.md](../theme/README.md) — the theme
>   editor's token model. SDUI renders against `starter-ui-kit` shadcn
>   primitives, which the theme editor restyles. The two are
>   independent: SDUI is the structure, the theme is the skin.
> - [DOCS/frontend/ai-builder/SCOPE.md](../ai-builder/SCOPE.md) — the
>   first authoring mode. AI sessions emit SDUI `ComponentTree`s into a
>   flow's output slot; SDUI is the wire format AI-builder targets.

## One-line summary

**SDUI** is starter's typed Server-Driven UI substrate: a Rust crate
holding a versioned component IR (~30 variants), a binding engine that
resolves `$target/child.slot` expressions against an entity graph, a
React renderer that projects the IR to shadcn primitives, and a
typed Rust builder DSL that lets consumers author pages from
`main.rs`. The server emits a typed component tree; the client renders
it. JSON in, pixels out. Every interaction round-trips through one
`/ui/action` endpoint.

Imported in bulk from the Rubix workspace's SDUI implementation
(`rubix-contracts/ui-ir`, `rubix-ui-core/src/sdui/`,
`extension-sdk/sdui-builder`) which has been validated through Rubix
phases S1–S7 against three falsification use cases (BACnet discovery,
PR review cards, scope-plan boards). Starter owns and maintains the
ported copy going forward; the Rubix version is no longer the source
of truth.

## Why this exists

Three things drive an SDUI substrate in starter:

1. **One renderer, every screen.** Every starter-based product that
   "shows business data" ends up writing the same CRUD-shaped screens
   — device pages, settings forms, alarm tables, audit logs, kind
   browsers — once per product, per client. A typed IR + dumb projector
   collapses that to one renderer for the long tail; specialist UX
   stays opt-in via Module Federation or the `custom` escape hatch.
2. **Cross-client parity by construction.** The IR is JSON + JSON
   Schema; it has no React in it. A Flutter renderer (deferred but not
   blocked) reads the same trees the React renderer reads. The
   alternative — AI-generating React *and* Flutter for every screen —
   produces two diverging implementations of every page and turns
   parity into a manual diff exercise.
3. **AI authoring lands cleanly on a typed format.** An LLM emitting
   `{ "type": "kpi", "label": "...", "value": "{{$target/temp.value}}" }`
   inside a stable schema is a tractable, validatable task. An LLM
   emitting freeform React for every product is a per-screen prompt
   loop forever. The [ai-builder](../ai-builder/SCOPE.md) capability
   targets this IR specifically; without SDUI, ai-builder degrades to
   "AI writes a hand-coded page per request."

This is the **second implementation** of this pattern — Rubix wrote
the first under the same name. Starter is not re-inventing; it is
adopting Rubix's vocabulary, IR shape, action protocol, and binding
grammar wholesale, with one substitution: render against
`starter-ui-kit` shadcn primitives instead of `@rubix/ui-core`'s.
Where Rubix's SDUI doc and starter's diverge, starter's wins for
starter consumers — but the IR wire shape stays compatible so the
ported renderer code largely "just works."

## Relationship to existing crates

```
starter-spi                       (Tool, AiRunner, Service — exists,
                                   unchanged)
   ▲
   │
   ├── starter-ui-ir              (NEW — ported from rubix-ui-ir.
   │                               The component enum, JSON Schema,
   │                               version stamp, `Bindable<T>` shim.
   │                               Zero I/O. R1.)
   │
   ├── starter-ui-bindings        (NEW — the binding grammar
   │                               (`$target`, `$stack`, `$user`,
   │                               `$page`, `$self`, `/` child walk),
   │                               `EvalContext`, subscription-plan
   │                               derivation. Depends on starter-ui-ir
   │                               + a host-supplied entity-graph trait.)
   │
   ├── starter-ui-builder         (NEW — ported from rubix-sdui-builder.
   │                               Typed Rust constructors for hand-
   │                               authored pages: layout(), table(),
   │                               kpi(), form(), rsql() builder, etc.
   │                               Produces ComponentTree values from
   │                               main.rs.)
   │
   ├── starter-sdui-routes        (NEW — opt-in axum routes:
   │                               POST /api/v1/ui/resolve,
   │                               POST /api/v1/ui/action,
   │                               GET  /api/v1/ui/table.
   │                               Consumer mounts via
   │                               sdui_router(state). starter-server
   │                               does NOT depend on this.)
   │
   ├── starter-ui-kit             (exists — shadcn primitives. SDUI
   │                               renderer in @nube/starter-ui-sdui-react
   │                               renders IR variants against these.)
   ├── starter-ui-core            (exists — react-query/zustand hooks.
   │                               SDUI live-subscription hook lands
   │                               here.)
   │
   ├── @nube/starter-ui-sdui-react   (NEW — ported from rubix-ui-core's
   │                               src/sdui/. <Renderer>, <SduiPage>,
   │                               16-30 component implementations,
   │                               action dispatcher, subscription
   │                               client, optimistic-patch helpers,
   │                               capability handshake. Renders against
   │                               starter-ui-kit primitives.)
   └── @nube/starter-ui-sdui-react-flutter
                                  (DEFERRED — Flutter port. IR is
                                   language-agnostic; only React in v1.)
```

There is no `starter-sdui` mega-crate. The Rust side splits into three
narrow crates (`ir`, `bindings`, `builder`) so consumers depend only on
what they need — a tool-only consumer authoring `ComponentTree`s in
`main.rs` pulls `starter-ui-ir` + `starter-ui-builder`; the binding
engine and the HTTP routes are opt-in.

## Hard rules

These are specific to SDUI. Flow and agent rules apply transitively.

### R1 — `starter-ui-ir` has no I/O deps

Types, schema, and version stamp only. No axum, no reqwest, no
tokio, no entity-graph trait. A mobile-safe consumer (a future
Flutter codegen, a test crate, a CLI that pretty-prints IR) must be
able to depend on it without dragging a runtime in.

**Enforcement is the transitive dep graph, not a `Cargo.toml`
grep.** CI runs `cargo tree -p starter-ui-ir --edges normal` and
fails if any of `axum`, `axum-core`, `reqwest`, `hyper`, `tokio`,
`tokio-util`, `tower`, `tower-http`, `h2`, `http-body` appears
anywhere in the transitive tree. A direct-deps grep misses the
common failure mode (a "harmless" `Cargo.toml` line pulls one of
the above through a feature flag on a serialisation crate).

### R2 — The IR is versioned; consumers handshake

The IR root carries `ir_version: u32`. The client advertises the
versions it supports in the capability response; the server clamps
emission to the highest mutually-supported version. The React
renderer refuses to project a tree whose `ir_version` exceeds
`SUPPORTED_IR_VERSION` and shows a mismatch banner rather than
guessing.

Adding a component variant is a minor bump. Removing or re-shaping a
variant is a major bump with a 12-month deprecation window. Same
discipline as REST API versioning.

The dual-field tolerance pattern (see Rubix's V5 chart migration
note) is the canonical way to evolve a variant: accept both old and
new field names for one release, default-write the new, drop the
old after the deprecation window. The TS Zod schemas and the React
renderer mirror this tolerance.

### R3 — The React app never knows what the domain is

Zero domain-specific strings in `starter-ui-ir`, `starter-ui-bindings`,
or `@nube/starter-ui-sdui-react`. The renderer knows about `table`,
`form`, `kpi`, `chart`, `diff`, `row`, `col`. It does not know about
the consumer's entities, regardless of what those entities are.

**Enforcement is structural, not a fixed denylist.** A keyword grep
(`building|device|alarm|pr|scope|...`) gives false comfort: the next
consumer's domain is `vehicle|policy|claim|gate` and the denylist
passes silently while R3 is violated. The check is:

1. **An allowlist of vocabulary** in `crates/starter-ui-ir/words.txt`
   and `packages/starter-ui-sdui-react/words.txt` — IR variant names, IR
   field names, framework concepts (`subscribe`, `optimistic`,
   `capability`), HTML / CSS terms, library names. Any string literal
   or identifier in those crates that is not in the allowlist, not
   under `tests/fixtures/`, and not in a comment is a CI failure.
2. **The allowlist is reviewed in PR.** Additions require one sentence
   in the PR description naming the framework concept the word
   represents. "Convenience" is not a framework concept.

A keyword denylist (the obvious approach) is also in CI as a
defence-in-depth tripwire — but it's the cheap check, not the real
one. The allowlist is the contract.

This is the load-bearing property that lets one renderer cover N
products: any domain, any vocabulary, same renderer, different IR
trees.

### R4 — Bindings resolve server-side; the client never sees expressions

The binding engine (`starter-ui-bindings`) evaluates `{{ ... }}`
expressions during resolve, against `EvalContext { target, stack,
user, page_state, self }` and an entity-graph trait the host
provides. The wire shape the client receives has every binding
substituted to a literal — except for `subscribe:` declarations,
which carry NATS-like subject strings the subscription client
applies live updates against.

The client cannot evaluate bindings. The grammar lives in one place
(server-side); the renderer is dumb projection. This is what keeps
the renderer at ~800 LoC and the IR semantics auditable.

### R5 — One action endpoint, discriminated response

Every interaction goes through `POST /api/v1/ui/action` with body
`{ handler, args, context }`. The response is the discriminated
union:

```
{"type": "patch",       "target_component_id": "...", "tree": { ... }}
{"type": "full_render", "tree": { ... }}
{"type": "navigate",    "to": { "target_ref": "..." }}
{"type": "toast",       "intent": "ok"|"warn"|"danger", "message": "..."}
{"type": "diagnostics", "items": [{ severity, code, message, field? }]}
{"type": "download",    "url": "..."}
{"type": "stream",      "channel": "..."}
{"type": "none"}
```

`form_errors` from Rubix's SDUI is renamed to `diagnostics` on
import; the wider `{severity, code, message, field?}` shape covers
warnings and info, not just per-field errors. Rubix's `form_errors`
back-compat is **not** ported — starter has not shipped, so we drop
it. A handler emitting `form_errors` is a parse error at the wire.

Handlers register by name in a `HandlerRegistry`. Extensions
contribute handlers. Auth/RBAC is enforced at the handler level via
the principal in `context`.

### R6 — Tables are queries, not row lists

`table` components carry a `source: { query, subscribe }` block, not
a rendered list of rows. The client receives an empty table and
issues `GET /api/v1/ui/table?source_id=...&page=...&sort=...` to
fetch a page; live updates arrive via the subscription plan the
resolver emitted alongside the tree. One component covers the
60-percent-of-every-app "list of things with paging, sort, filter,
click-through" pattern without per-screen code.

The query grammar is **RSQL** (per Rubix's choice; the builder DSL
exposes a typed `rsql()` builder). The query engine implementation
is a host concern — starter ships an in-memory reference impl for
examples; production consumers wire their own.

### R7 — `custom` is the escape hatch, not a feature

When the IR vocabulary doesn't cover what a consumer needs, the
`custom` variant defers rendering to a block-registered React
component:

```json
{ "type": "custom",
  "renderer_id": "com.acme.floorplan",
  "props": { ... },
  "subscribe": [ ... ] }
```

The React app looks up `renderer_id` in a client-side registry
(populated at Module Federation load time, or imported directly in
shadcn-only consumers). Unknown `renderer_id` degrades to a neutral
stub; the rest of the page renders.

`custom` is in v1, not deferred. It covers floor plans, gauges,
schedule grids, anything domain-specific. The server filters
`custom` nodes against the client's advertised renderer-id set
before emission (capability handshake); an unfamiliar id is
rewritten to a `dangling` stub server-side and never reaches the
client live.

**Threat model for the capability handshake.** A malicious or
curious client can lie about which `renderer_id`s it supports —
either to **harvest schemas** by advertising every plausible id and
inspecting the returned `custom` props, or to **probe for features**
("does this server know about `com.acme.secret`?"). Two rules
contain that:

1. **`renderer_id` is treated as public.** Its existence in any
   capability response is not a secret. If the existence of an id
   leaks deployment information (e.g. "this server has the
   internal-admin floorplan widget"), gate the *deployment* — don't
   try to hide the id from a capability check.
2. **`custom.props` are scoped to the renderer's contract, not the
   user's permissions.** A handler emitting `{ "type": "custom",
   "renderer_id": "com.acme.report", "props": { sensitive_data } }`
   is responsible for ensuring the props are appropriate for the
   `Principal` the resolve was issued against. The capability filter
   is a *vocabulary* check ("does this client know how to render
   this id"), not an *authorisation* check ("is this user allowed
   to see this data"). Conflating the two is a bug — auth runs at
   the handler boundary per [R5](#r5--one-action-endpoint-discriminated-response)
   and at the resolve boundary, both before the `custom` node is
   ever constructed.

A renderer whose props contain secrets when rendered for any
plausible principal is misconfigured at the source.

This is the rule that lets SDUI be the **default** path without
being a **mandate**: any screen the IR can't express is a `custom`
renderer-id or a normal Module Federation route, not an SDUI gap.

### R8 — Size and DoS limits are enforced server-side

| Limit | Value | Evidence |
|---|---|---|
| Max IR tree nodes per resolve | **2000** | **Inherited / unmeasured.** Rubix S7 pinned the limit with a fixture; no production data behind the value. |
| Max tree depth | **32** | **Inherited / unmeasured.** Practical pages observed at depth ≤ 8; 32 is "obvious headroom." |
| Max serialized tree | **2 MiB** | **Reused.** Aligns with existing render-tree cap in `starter-server`. Real workload alignment, not measurement of an SDUI page. |
| Max distinct component types per page | **60** | **Inherited / unmeasured.** Total IR vocabulary is ~30; 60 leaves room for `custom` ids. |
| Max action handler timeout | **5 s** (server-side enforcement) | **Inherited / unmeasured.** Server cancels the handler future at this deadline and returns a `diagnostics` error; client may give up sooner but that's its policy. |
| Max rows per table page | **500** | **Inherited.** Above ~500 rows the client-side virtualisation Rubix shipped showed measurable jank in S6 testing. |
| `page_state` byte cap | **64 KiB** | **Inherited / unmeasured.** Chosen as "comfortably above any realistic `chart_range`-shaped payload." |

A violation returns `413 Payload Too Large` with a stable `what:`
tag naming the limit. Tests pin each tag (the *enforcement* is
covered; the *limit value* is not).

**These are starting points, not load-tested numbers.** The first
consumer that hits one of the "inherited / unmeasured" rows is the
signal to re-measure, not to widen reflexively. When a limit moves,
update its row above with the measurement and the workload that
justified it. The goal is for this table to mostly say "measured
against [X workload]" within two consumer adoptions; if it doesn't,
we are flying blind.

### R9 — No client-side business logic

Optimistic-patch hints exist for UX latency (`optimistic: { patch:
{...} }` on a button applies immediately, server response confirms
or replaces) — but the authoritative response is always the
server's. No client-side validation library, no client-side
permission check, no client-side derived state. The React app is a
projector.

A consumer that needs a richer client (a node-graph editor, a
floor-plan designer) ships it as a `custom` renderer or a full MF
route — both already-supported escape hatches that don't pollute
the IR.

## Authoring modes

A `ComponentTree` can be produced four ways. All four are
first-class.

| Mode | Who authors | When |
|---|---|---|
| **Rust builder DSL** (`starter-ui-builder`) | Block / product author, in `main.rs` or a `seed()` call | Compile time |
| **Hand-written JSON / YAML** | Power user editing a config file | At any time |
| **AI (ai-builder)** | LLM session emitting trees into a flow output slot | At runtime |
| **Drag-drop visual editor** | End user in a Studio canvas | Deferred — Phase 5+ |

The renderer doesn't know or care who authored the tree. The Rubix
work proved this works: the same `Renderer` projects AI-authored,
hand-authored, and DSL-authored pages identically.

The builder DSL is the primary authoring mode for *starter*
consumers — most CRUD screens are easier to type out as Rust than to
prompt an LLM for. AI is the right tool for one-off / exploratory
pages and for end-user-driven authoring; the DSL is the right tool
for the spine of a product.

## Surface — Rust (builder DSL)

```rust
use starter_ui_builder::prelude::*;

pub fn building_overview() -> ComponentTree {
    page("building-overview", "{{$target.name}} Overview", [
        kpi_grid([
            kpi("outdoor").label("Outdoor Temp")
                .value(target().child("outdoor-temp").slot("value"))
                .unit(target().child("outdoor-temp").slot("units")),
            kpi("energy").label("Energy (kWh)")
                .value(target().child("kwh").slot("value")),
        ]),
        table("alarms",
              rsql()
                  .parent_path_prefix("{{$target.path}}/alarms")
                  .kind("alarm.active"))
            .column("Time", "slots.ts.value")
            .column("Severity", "slots.severity.value")
            .live()
            .build(),
    ])
}
```

**What is compile-time-checked, and what isn't.** The builder uses
newtype source/kind pairing (Rubix's `TimeSeriesSource` /
`RowsSource` pattern): passing a `RowsSource` to `line_chart` is a
build error. Component-level shape errors (a `kpi` without a
`value`) are also compile-time, by the type system.

**Binding strings are not compile-time-checked.** A typo in
`target().child("outdoor-temp")` produces a valid `String` that the
binding engine will fail to resolve at request time, returning a
structured `BindingError`. Validating bindings at compile time would
require `starter-ui-builder` to depend on `starter-ui-bindings` (so
it could parse the grammar) and on a per-consumer `EntityGraph`
shape (so it could verify the child / slot exists) — the first
would couple two crates the dependency split is built to keep
separate; the second is impossible without consumer-specific
generics. The trade is intentional: source/kind get compile-time
safety, binding strings get resolve-time errors with line-numbered
diagnostics.

The dependency split holds: `starter-ui-builder` depends only on
`starter-ui-ir` (for the types it constructs), not on
`starter-ui-bindings`. A consumer authoring pages-as-code from
`main.rs` pulls `ir + builder`; the binding engine ships on the
server.

The builder is **the** maintenance interface for starter's own
example pages and for any consumer who wants pages-as-code. AI
authoring (ai-builder) emits the same `ComponentTree` shape over the
wire; the two converge on identical artifacts.

## Surface — Rust (HTTP routes, opt-in)

```rust
use starter_server::ServerBuilder;
use starter_sdui_routes::{sdui_router, SduiState};   // opt-in crate

let state = SduiState::builder()
    .with_entity_graph(my_graph)           // implements EntityGraph trait
    .with_query_engine(my_query)           // implements QueryEngine trait
    .with_handler_registry(handlers)       // HandlerRegistry
    .build()?;

ServerBuilder::<AppState>::new(state)
    .merge_router(sdui_router::<AppState>(state))
    .build()
    .serve()
    .await?;
```

`sdui_router` mounts the three endpoints:

- `POST /api/v1/ui/resolve` — resolve a page ref + target to a
  `ComponentTree`. Body: `{ page_ref, target_ref, stack?, page_state? }`.
  Response: `{ render: ComponentTree, subscriptions: [...] }`.
- `POST /api/v1/ui/action` — dispatch a named handler. Body:
  `{ handler, args, context }`. Response: the action-response union.
- `GET /api/v1/ui/table` — paginated table source.
  Query: `?source_id=...&page=...&size=...&sort=...&filter=...`.

**Feature flag specification.** The three routes live in a
separate crate `starter-sdui-routes` rather than behind a feature
flag on `starter-server`, because Cargo features on `starter-server`
cannot prevent the underlying `starter-ui-ir` /
`starter-ui-bindings` / `starter-ui-builder` crates from being
built — workspace crates compile if anything in the workspace
depends on them.

The split is therefore at the **consumer's `Cargo.toml`**, not at a
feature flag:

- A consumer that wants SDUI HTTP routes adds
  `starter-sdui-routes = "0.1"` and calls `sdui_router(...)`.
- A consumer authoring pages-as-code only adds `starter-ui-ir` +
  `starter-ui-builder` and never pulls the binding engine, the
  routes crate, or `axum`.
- A consumer wanting custom transport (Tauri IPC, gRPC, a CLI
  pretty-printer) depends on `starter-ui-ir` + `starter-ui-bindings`
  and drives the resolver themselves; no HTTP code compiled.

`starter-server` itself never depends on any of the four. Adding a
direct dep from `starter-server` to `starter-sdui-routes` would
force every consumer that uses `starter-server` to compile the
SDUI dep graph regardless of whether they use it — the wrong
default. The routes crate is opt-in by the consumer's Cargo.toml,
period.

## Surface — React

```tsx
import { SduiProvider, Renderer } from "@nube/starter-ui-sdui-react";
import { StarterClient } from "@nube/starter-client-ts";

const client = new StarterClient({ baseUrl: "/" });

export function App() {
  return (
    <SduiProvider client={client}>
      <Routes>
        <Route path="/ui/:pageRef" element={<SduiPage />} />
        <Route path="/render/:targetId" element={<SduiRenderPage />} />
      </Routes>
    </SduiProvider>
  );
}
```

`<SduiPage>` and `<SduiRenderPage>` handle the resolve round-trip,
the live subscription wiring, the action dispatcher, and the
optimistic patch helpers. Custom renderers register through:

```tsx
import { registerCustomRenderer } from "@nube/starter-ui-sdui-react";
registerCustomRenderer("com.acme.floorplan", FloorPlanComponent);
```

Size targets (red lines, ported from Rubix SDUI):

- Core renderer (`Renderer.tsx`, dispatch + recursion + action
  client): **≤ 800 lines TSX, single file**.
- Built-in component implementations: **~3000 lines total across
  all components**, **4000 lines red line, total**. Per-component
  files vary in size — a `kpi` is short, a `table` with virtualised
  scroll is long; the budget is the sum.
- `diff` and `rich_text` delegate to monaco-diff and tiptap/milkdown;
  only the IR-adapter wrappers (the file mapping IR fields to
  library props) count toward the budget. The libraries themselves
  do not.

Crossing the red line means we've over-engineered or we're missing a
`custom` escape hatch; review before merging.

## Data bindings

The grammar ports from Rubix DASHBOARD.md verbatim:

```
binding := source ( "." ident                # slot read on current cursor
                  | "/" ident                # child node by name (cursor move)
                  )*
source  := "$target" | "$stack" "." alias | "$self"
         | "$user" | "$page"
```

- `.` is **data access** — read a slot on the cursor's current node.
- `/` is **graph traversal** — move the cursor to a named child.

`{{$target/outdoor-temp.value}}` = "navigate to the child named
`outdoor-temp` under `$target`, then read its `value` slot."

The grammar's load-bearing property: **one page node × N targets = N
live dashboards**. A page that says `"value": "{{$target/temp.value}}"`
renders correctly against `building-1`, `building-2`,
`building-N` with zero duplication, and the subscription plan
isolates live updates per target automatically.

Without this, AI-generated pages devolve into hardcoded snapshots
that need re-generating per entity. With it, one AI session produces
one page that covers every entity of its target kind.

## What does NOT land

- **Offline-first.** Online-only in v1. The cache layer that enables
  offline is in scope eventually; full offline mode is not.
- **Client-side business logic.** R9. Every interaction round-trips.
  Optimistic hints are the only client-side state mutation.
- **A full layout engine.** IR layout variants (`row`, `col`, `grid`,
  `stack`, `split`, `tabs`, `scroll`, `spacer`) map 1:1 to flex/grid;
  no bespoke algorithm.
- **A theme system inside IR.** The theme lives in
  [`starter-ui-kit`](../theme/README.md). IR carries semantic hints
  (`intent: "danger"`, `size: "lg"`) not CSS.
- **Sucrase / JSX-over-wire.** Rubix SDUI deferred this; starter
  inherits the deferral. Typed IR + `custom` escape hatch is
  expressive enough.
- **Drag-drop visual page editor.** Phase 5+; not v1. The builder DSL
  and AI authoring cover v1.
- **Flutter renderer.** Deferred. The IR is language-agnostic; a
  starter consumer asking for it triggers the port.
- **Per-screen feature flags / A/B tests at IR emission.** Tractable
  later; not v1.

## Smoke tests (before merging)

Ported and adapted from Rubix SDUI's acceptance criteria.

### "Domain-leak structural check" test (R3)

The CI script scans `crates/starter-ui-ir/src/`,
`crates/starter-ui-bindings/src/`, and
`packages/starter-ui-sdui-react/src/` for any string literal or
identifier (in source, not comments / tests / fixtures) that is not
present in the per-crate `words.txt` allowlist. Any unlisted token
fails the build with a pointer to the file + line.

A keyword denylist (`building|device|alarm|pr|scope|...`) runs
alongside as a defence-in-depth tripwire — but the allowlist is the
contract, since a denylist passes silently for any consumer whose
domain isn't on it.

### "One page, N targets" test

A fixture `ui.page` with `{{$target/temp.value}}` bindings resolves
correctly against three different target nodes; each resolve produces
distinct literals; the per-resolve subscription plan scopes subjects
to the resolved target. (Ports Rubix's worked example.)

### "Capability mismatch refuses to render" test (R2)

A V+1 tree against a V client returns a clean mismatch banner from
`<SduiPage>` and never reaches the dispatcher. The handshake
endpoint advertises supported versions; emission clamps.

### "Action handler 404 is structured" test (R5)

`POST /api/v1/ui/action` with an unregistered handler returns 404
with body `{ "type": "diagnostics", "items": [{ severity: "error",
code: "handler_not_found", message: "...", field: null }] }`. The CLI
exit code (if invoked via `agent`) matches.

### "Table pagination round-trip" test (R6)

A `table` with a 10k-row source renders empty on resolve; the client
issues one `/ui/table` request per page; sort and filter each
produce one round-trip; the client virtualises row rendering and
never re-fetches on scroll within a fetched page.

### "Custom renderer falls back cleanly" test (R7)

A tree with `{ type: "custom", renderer_id: "unknown.id" }` renders
a neutral stub component; the rest of the tree renders normally;
the console logs a structured warning naming the unknown id.

### "DoS limit returns 413 with `what:` tag" test (R8)

Each of the seven limits in R8 has a fixture that violates it; each
returns 413 with the expected stable tag. Ports
`crates/dashboard-transport/tests/limits.rs` from Rubix.

### "Builder DSL produces valid IR" test

Every public builder function in `starter-ui-builder::prelude`
produces a `ComponentTree` that round-trips through the IR's JSON
Schema validator without error.

### "Falsification: SDUI covers three diverse domains"

Three fixture pages — a device list (CRUD), a PR review card (`diff`
+ inline actions), and a scope board (state badges + live updates)
— render end-to-end through one renderer with zero domain-specific
strings in the renderer crates. Ports Rubix's UC1 / UC2 / UC3.

## Phasing

The port from Rubix is structurally a copy-paste with a rename, but
the integration with starter's seams (entity graph, query engine,
auth, server router) requires real work. Each phase is one session.

| # | Phase | Size | Output |
|---|---|---|---|
| 1 | `starter-ui-ir` port + JSON Schema + version stamp | S | Crate compiles, schema emits, round-trips through serde, V5 chart dual-field tolerance preserved. Unit-tested in isolation. |
| 2 | `starter-ui-bindings` port — grammar, EvalContext, subscription-plan derivation, EntityGraph trait | M | Worked-example test ("one page, N targets") passes against a fixture entity graph. |
| 3 | `starter-ui-builder` port — typed constructors, `rsql()` builder, `seed_page()` | M | Builder DSL produces valid IR; example page authored from main.rs renders end-to-end against fixture data. |
| 4 | `@nube/starter-ui-sdui-react` port — `<Renderer>`, `<SduiPage>`, ~16 component implementations against `starter-ui-kit` primitives | L | Core renderer + 16 components under size budget; route `/ui/:pageRef` renders any authored page. Domain-leak grep passes. |
| 5 | `starter-server` SDUI routes (`/resolve`, `/action`, `/table`) behind feature flag; `HandlerRegistry`; capability handshake | M | Action 404 test passes; pagination round-trip test passes; capability-mismatch test passes. |
| 6 | Remaining IR components (`chart`, `sparkline`, `tree`, `timeline`, `markdown`, `wizard`, `drawer`, `rich_text`, `diff`) + streaming `text`/`markdown` via subscription | L | All R8 DoS limits enforced with stable tags; falsification suite (CRUD + diff + state-board) passes. |
| 7 | `custom` escape-hatch wiring (registry, capability filter, fallback stub) | S | Custom-renderer fallback test passes; unknown `renderer_id` rewrites to `dangling` server-side per R7. |
| 8 | Optimistic action hints + diagnostics response shape + DoS limit tests | M | Optimistic-patch round-trip < 16 ms; `form_errors` parse-rejected at wire; full acceptance suite green. |

Phases 1–5 are the MVP — a usable SDUI substrate authored via the
Rust DSL only. Phases 6–8 close the vocabulary and the safety surface.
ai-builder Phase 2 ([ai-builder SCOPE](../ai-builder/SCOPE.md)) is
blocked on Phase 5 here landing.

## Decisions made

- **Port wholesale; rename on import.** `rubix-ui-ir` →
  `starter-ui-ir`, `rubix-sdui-builder` → `starter-ui-builder`,
  `rubix-ui-core/src/sdui/` → `@nube/starter-ui-sdui-react`. Internal
  type names keep their Rubix-side spelling where the IR wire shape
  is visible (`ComponentTree`, `Component`, `ChartSource`) so a
  developer reading Rubix's SDUI.md and starter's source sees the
  same words.
- **`form_errors` → `diagnostics`, no back-compat.** Rubix kept
  `form_errors` deserialising for one release; starter hasn't
  shipped, so we drop it at the wire. Cleaner schema, no migration
  story.
- **Render against `starter-ui-kit`, not `@rubix/ui-core`.** The IR
  is the contract; the renderer is implementation. Replacing
  Rubix's render targets with shadcn primitives is the one
  substantive change vs. a pure copy-paste.
- **Three Rust crates, not one.** `ir` / `bindings` / `builder` split
  so a consumer authoring static pages in `main.rs` doesn't pull the
  binding engine; a CLI pretty-printer doesn't pull the builder.
- **HTTP routes behind a `feature = "sdui"` flag.** Consumers that
  don't expose SDUI routes don't compile them.
- **AI authoring (ai-builder) targets this IR.** Confirmed by
  rewriting [ai-builder SCOPE](../ai-builder/SCOPE.md) to emit
  `ComponentTree` deltas instead of inventing a `UiTree` /
  `BuilderEvent` wire format.

## Decisions

The five questions under [Open questions](#open-questions) are
pinned here with the working decision and the signal that should
re-open them. The Open questions section below remains as the
narrative rationale; this section is the load-bearing record. If
this section and the Open questions section disagree, this
section wins.

| ID | Decision | Revisit trigger |
|---|---|---|
| **S-D1** | `EntityGraph` trait lives in `starter-ui-bindings`. Consumers implement against their own graph; no SPI promotion in v1. | A **second** consumer (beyond the first SDUI adopter) asks for the trait, or a non-bindings crate needs to name the trait in its public API. Promotion to `starter-spi` is mechanical; demotion isn't, so we wait for the second use. |
| **S-D2** | RSQL query engine is a **trait + in-memory reference impl** in v1, shipped in `starter-sdui-routes` (or a sibling) for examples and tests. Production consumers wire their own backend against the trait. No port of Rubix's `crates/query`. | First production consumer hits the in-memory impl's limits (dataset size, push-down requirements, transactional reads), **or** a second consumer would otherwise reimplement the same backend adapter. Either signal promotes the engine to its own crate. |
| **S-D3** | Visual drag-drop page editor is **deferred**. Builder DSL + AI authoring + hand-written JSON cover v1 authoring. Lands as a `starter-extension` (separate repo / MF bundle) when demanded. | A consumer explicitly requests end-user-driven page authoring inside a running product (not power-user JSON editing, not AI prompt-driven). At that point, scope a starter-extension; do not absorb into the SDUI core crates. |
| **S-D4** | `oneOf` server-resolved variant sub-form helper lives on the **renderer side** (`@nube/starter-ui-sdui-react`). The server emits the active variant; the renderer renders it like any other sub-form. No `oneOf` resolution logic in `starter-ui-builder`. | The renderer-side helper grows past ~200 LoC, **or** a non-React renderer (Flutter port) needs to re-implement the same variant-selection logic. Either signal pushes the helper down to `starter-ui-bindings` or `starter-ui-ir`. |
| **S-D5** | Stream end-of-stream sentinel inherits Rubix verbatim: `{ "type": "stream_end", "channel": "...", "reason": "done"\|"error"\|"timeout"\|"gone" }`. Same field names, same reason values, no rename. | A reason value Rubix didn't define is needed (e.g. `"cancelled"`, `"backpressure"`), **or** the wire shape needs to carry per-stream diagnostics. Extending the `reason` enum is additive; reshaping the sentinel is a major IR bump per R2. |

These decisions block code: Phase 1 lands assuming S-D1 placement,
Phase 5 lands assuming S-D2 shape, and Phases 4 / 6 lands assuming
S-D4 / S-D5. A revisit trigger firing is a scope change, not a
silent refactor — update this table in the same PR.

## Open questions

### S-D1 — Where the entity graph trait lives

`starter-ui-bindings` needs an `EntityGraph` trait to walk
`$target/child.slot`. Two options:

- **(a)** Trait in `starter-ui-bindings`; consumers implement against
  their own graph.
- **(b)** Trait in `starter-spi` next to `Tool` / `AiRunner`,
  signalling it as a starter-wide seam.

Lean (a) until a second consumer wants the trait. Promotion to SPI
is mechanical; demotion isn't.

### S-D2 — RSQL query engine: ship a default or require BYO?

Rubix's `crates/query` is its own crate doing real work (RSQL
parsing, push-down to whatever backend). Porting it is a separate
project. v1 option: ship a trait + an in-memory reference impl
sufficient for examples; production consumers wire their own
backend. Decide at Phase 5 entry gate.

### S-D3 — Visual page editor (drag-drop)

Rubix defers this to "Studio Stage 4+ work." Starter has no Studio.
Likely lands as a starter-extension (separate repo, MF bundle) when
a consumer demands it. Not blocking.

### S-D4 — Schema-driven forms

Rubix's `form` component derives every field from a JSON Schema via
a fixed mapping table (~150 LoC). Starter inherits the table
verbatim. Open: where does the `oneOf` server-resolved variant
sub-form helper live — in `starter-ui-builder` or in the renderer?
Lean renderer; the server emits the active variant, the renderer
renders it like any other sub-form.

### S-D5 — Streaming sentinel naming

Rubix uses `{ "type": "stream_end", "channel": "...", "reason":
"done"|"error"|"timeout"|"gone" }` as the end-of-stream sentinel.
Inherit verbatim — no reason to bikeshed.

## Pointers

- **Drift log** — every intentional divergence from Rubix lives here
  and gets updated in the same PR as the divergence:
  [DOCS/frontend/sdui/DIVERGENCE.md](./DIVERGENCE.md)
- **This file is the normative spec.** The Rubix references below
  are the **origin**, not the spec. When this doc and Rubix's
  SDUI.md disagree on a wire field or rule, this doc wins for
  starter consumers. See DIVERGENCE.md for the diff.
- Rubix SCOPE (origin, useful for understanding lineage and the
  worked examples):
  [`rubix-workspace/rubix-agent/docs/design/frontend/SDUI.md`](file:///home/user/code/rubix-workspace/rubix-agent/docs/design/frontend/SDUI.md)
- Rubix IR crate ported from:
  [`rubix-workspace/rubix-contracts/ui-ir/`](file:///home/user/code/rubix-workspace/rubix-contracts/ui-ir/)
- Rubix React renderer ported from:
  [`rubix-workspace/rubix-ui-core/src/sdui/`](file:///home/user/code/rubix-workspace/rubix-ui-core/src/sdui/)
- Rubix builder DSL ported from:
  [`rubix-workspace/extension-sdk/sdui-builder/`](file:///home/user/code/rubix-workspace/extension-sdk/sdui-builder/)
- First downstream consumer:
  [DOCS/frontend/ai-builder/SCOPE.md](../ai-builder/SCOPE.md)
- Theme editor (independent — restyles SDUI's render output):
  [DOCS/frontend/theme/README.md](../theme/README.md)

### Scrub plan

The body of this doc currently references "Rubix" ~30 times,
treating it as the source of truth for design decisions ("ported
verbatim from Rubix § X"). That framing is honest for the port
phase but becomes dead weight once starter's SDUI has diverged
materially. **Scrub trigger**: once the [DIVERGENCE](./DIVERGENCE.md)
table grows past ~6 entries, do a pass over this file and replace
"ported from Rubix" with "starter's …" plus a one-line origin note
in the appropriate section. The pointers above stay; the in-line
appeals to Rubix authority go.

## Bottom line

**SDUI is starter's typed UI substrate.** A versioned component IR
in `starter-ui-ir`, a binding grammar + subscription planner in
`starter-ui-bindings`, a typed Rust builder in
`starter-ui-builder`, a React renderer against `starter-ui-kit`
primitives in `@nube/starter-ui-sdui-react`, three HTTP routes in
`starter-server` behind a feature flag. Imported from Rubix, owned
and maintained here. AI authoring (ai-builder), the Rust builder
DSL, hand-written JSON, and a future visual editor all converge on
the same `ComponentTree` shape — one renderer, one contract, many
authoring paths.
