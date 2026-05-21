# Scope — starter-sdui

> Source of truth: [`DOCS/frontend/sdui/SCOPE.md`](../../../DOCS/frontend/sdui/SCOPE.md)
> in the starter repo, with intentional divergences from the Rubix
> origin tracked in
> [`DOCS/frontend/sdui/DIVERGENCE.md`](../../../DOCS/frontend/sdui/DIVERGENCE.md).
> The Rubix references (`rubix-workspace/rubix-contracts/ui-ir`,
> `rubix-workspace/rubix-ui-core/src/sdui/`,
> `rubix-workspace/extension-sdk/sdui-builder`) are the **origin**,
> not the spec. When this file or the source-of-truth SCOPE
> disagrees with Rubix on a wire field or rule, starter's wins for
> starter consumers. This per-job brief is intentionally short.
> When this file disagrees with the source-of-truth SCOPE, that
> doc wins — open an issue and update this file.

## Goal

Ship the **SDUI substrate** into starter: a typed component IR
(`starter-ui-ir`), a binding grammar + subscription planner
(`starter-ui-bindings`), a typed Rust builder DSL
(`starter-ui-builder`), an opt-in axum router crate
(`starter-sdui-routes`), and a React renderer package
(`@nube/starter-sdui-react`) targeting `@nube/starter-ui-kit`
shadcn primitives. Imported in bulk from the Rubix workspace's
SDUI implementation (validated through Rubix phases S1–S7 against
three falsification use cases — BACnet discovery, PR review
cards, scope-plan boards). Starter owns and maintains the ported
copy going forward; the Rubix version is no longer the source of
truth.

The substrate unblocks ai-builder
([`DOCS/frontend/ai-builder/SCOPE.md`](../../../DOCS/frontend/ai-builder/SCOPE.md)):
its Phase 1 (`starter-ai-builder-prompt`) depends on the IR JSON
Schema artifact from this job's Phase 1; its Phase 2
(`starter-flow-node-ai-builder`) depends on the compiled
`starter-ui-ir` crate plus the `EntityGraph` trait from this
job's Phase 2; its end-to-end demo additionally needs this job's
Phase 5 (HTTP routes + capability handshake).

**Three Rust crates with an explicit dependency contract** —
`starter-ui-ir` has no I/O deps; `starter-ui-bindings` depends on
`starter-ui-ir` plus an `EntityGraph` trait; `starter-ui-builder`
depends only on `starter-ui-ir` (NOT on `starter-ui-bindings`,
per the M4 fix — binding strings get resolve-time errors, not
compile-time). **One React package** with strict size budgets.
**One opt-in routes crate** the consumer mounts themselves (NOT a
feature flag on `starter-server`, per the M6 fix — Cargo features
can't prevent transitive compilation; only a separate crate can).

## In scope

- **`crates/starter-ui-ir`** — workspace member depending only on
  `serde + serde_json + schemars + thiserror + tracing`. Ports
  `rubix-contracts/ui-ir` verbatim: `Component` enum,
  `Bindable<T>` shim, chart source / kind newtypes
  (`TimeSeriesSource`, `RowsSource`), version stamp
  (`ir_version: u32`). Build step emits the JSON Schema artifact
  so consumers (including `starter-ai-builder-prompt`) can depend
  on the schema file before the compiled crate ships. CI gate
  via `cargo tree -p starter-ui-ir --edges normal` denylists
  `axum, axum-core, reqwest, hyper, tokio, tokio-util, tower,
  tower-http, h2, http-body` per R1.
- **`crates/starter-ui-bindings`** — workspace member depending on
  `starter-ui-ir`. Ports the binding grammar from
  `rubix-agent/crates/dashboard-runtime`: `$target / $stack /
  $user / $page / $self` sources, `.` slot-read, `/` child-walk.
  Defines the `EntityGraph` trait (per S-D1 bias: trait in
  `starter-ui-bindings` until a second consumer wants it
  promoted) with `read_children`, `read_slot`, and an optional
  `entity_id_regex` method (returns `None` for hosts without a
  stable id format — needed by ai-builder R7). Implements
  `EvalContext` and subscription-plan derivation.
- **`crates/starter-ui-builder`** — workspace member depending
  only on `starter-ui-ir` (NOT on `starter-ui-bindings`, per the
  M4 fix). Ports `extension-sdk/sdui-builder` verbatim: typed
  constructors for layout / display / chart / data / input /
  form variants; `TimeSeriesSource` / `RowsSource` newtype
  pairing so passing a `RowsSource` to `line_chart` is a build
  error; `rsql()` builder; `bindings::{target, stack, user,
  self_, page_state, vars}` helpers; `seed_page()` idempotent
  upsert. Crate-level docs document the compile-time-vs-resolve-
  time contract verbatim: source/kind get compile-time safety,
  binding strings get resolve-time errors.
- **`crates/starter-sdui-routes`** — workspace member providing
  three axum routes mounted via `sdui_router(SduiState)`:
  - `POST /api/v1/ui/resolve` — `{ page_ref, target_ref,
    stack?, page_state? }` → `{ render: ComponentTree,
    subscriptions: [...] }`.
  - `POST /api/v1/ui/action` — `{ handler, args, context }` →
    discriminated union (`patch | full_render | navigate | toast
    | diagnostics | download | stream | none`).
  - `GET /api/v1/ui/table` — `?source_id=...&page=...&size=...
    &sort=...&filter=...`.
  - `HandlerRegistry`, `QueryEngine` trait, `EntityGraph` trait
    bound at builder time. **`starter-server` does NOT depend on
    this crate** — consumers opt in via their own `Cargo.toml`.
- **`packages/starter-sdui-react`** — npm workspace package
  depending on `react + @tanstack/react-query + zustand +
  @nube/starter-ui-kit + @nube/starter-ui-core`. Ports
  `rubix-ui-core/src/sdui/` structure: `SduiProvider`,
  `Renderer.tsx` (single file, ≤ 800 lines TSX — size budget is
  a CI gate), `SduiPage.tsx`, `SduiRenderPage.tsx`, `types.ts`,
  `registry/`, `applyPatch.ts`, `useActionResponse.ts`,
  `useSubscriptions.ts`, `useBoundWrite.ts`, `row-bind.ts`,
  `dialog-bus.ts`, `capability.ts`. 16 initial component
  implementations against `@nube/starter-ui-kit` shadcn
  primitives (NOT Rubix's primitives — divergence D2).
  Components total ≤ 3000 lines target / 4000 lines red line
  across all components combined.
- **Extended IR vocabulary in Phase 6**: `chart` (with
  multi-series `sources: Vec<ChartSource>` plus dual-field
  tolerance for `source` vs `sources` per the V5 migration),
  `sparkline`, `tree`, `timeline`, `markdown` (streaming
  `mode: append | replace`), `wizard`, `drawer`, `rich_text`
  (delegating to tiptap/milkdown — only the IR-adapter wrapper
  counts toward the LoC budget), `diff` (delegating to
  monaco-diff — only the wrapper counts; annotations +
  `line_action` per SCOPE "Diff interactions"), `ref_picker`,
  `date_range`.
- **Custom escape-hatch wiring in Phase 7**: client-side
  `registerCustomRenderer` registry; server-side capability
  filter rewriting unfamiliar `renderer_id`s to a `dangling`
  stub before emission (R7); fallback stub component renders
  neutral placeholder without crashing the tree.
- **Optimistic action hints + diagnostics shape in Phase 8**:
  `Action.optimistic` applied via React-Query `setQueryData`
  before round-trip; authoritative responses replace through
  `applyPatch`; rollback on error. `diagnostics` response variant
  with the wider `{ severity, code, message, field? }` shape.
  `form_errors` rejected at the wire (no back-compat per D1).
- **Server-side DoS limits in Phase 8**: all seven R8 limits
  enforced with stable `what:` tags
  (`page_state_bytes` 64 KiB, `render_tree_bytes` 2 MiB,
  `tree_nodes` 2000, `tree_depth` 32, `component_types` 60,
  `handler_timeout` 5 s server-side, `table_rows_per_page` 500).
  Each limit row in the SCOPE R8 table marked with its evidence
  column value per the M3 fix.
- **Structural domain-leak enforcement** — per-crate `words.txt`
  allowlist files committed under `crates/starter-ui-ir/words.txt`,
  `crates/starter-ui-bindings/words.txt`,
  `packages/starter-sdui-react/words.txt`. CI scans those crates
  for any string literal or identifier not present in its
  `words.txt`; unlisted token fails the build. Keyword denylist
  runs alongside as defence-in-depth, but the allowlist is the
  contract (per the M1 fix).
- **`DOCS/frontend/sdui/DIVERGENCE.md`** updated in the same PR
  as each divergence: D1 (form_errors → diagnostics), D2 (render
  target swap), D3 (three-crate split), D4 (separate routes
  crate), D5 (EntityGraph trait abstraction).
- **All ten SCOPE smoke tests** passing in CI plus the
  structural allowlist check, the size-budget CI gate, and the
  dep-tree CI gate.

## Out of scope

- **Offline-first.** Online-only in v1. The cache layer that
  enables offline is in scope eventually; full offline mode is
  not.
- **Client-side business logic.** R9 — every interaction
  round-trips. Optimistic hints are the only client-side state
  mutation.
- **A full layout engine.** IR layout variants map 1:1 to
  flex/grid; no bespoke algorithm.
- **A theme system inside IR.** Theming lives in
  `starter-ui-kit`. IR carries semantic hints (`intent:
  "danger"`, `size: "lg"`), not CSS.
- **Sucrase / JSX-over-wire.** Deferred from Rubix; starter
  inherits the deferral.
- **Drag-drop visual page editor.** Likely lands as a starter
  extension (separate repo, MF bundle) when a consumer demands
  it — not blocking.
- **Flutter renderer.** Deferred. The IR is language-agnostic;
  a starter consumer asking triggers the port.
- **Per-screen feature flags / A/B tests at IR emission.**
  Tractable later; not v1.
- **A vector / embedding selector or AI authoring inside this
  crate.** AI authoring is the [ai-builder
  job](../starter-ai-builder/)'s scope and depends on this job
  landing.
- **Editing rubix-workspace files.** This job ports the Rubix
  code into starter; the Rubix copy stays where it is. We are
  not maintaining a fork upstream.
- **A `starter-sdui` mega-crate.** Three narrow crates is the
  decision — D3.
- **A direct dep from `starter-server` onto
  `starter-sdui-routes`.** R6 / M6 — consumers mount via their
  own Cargo.toml.

## Hard rules (load-bearing)

- **R1** — `starter-ui-ir` has no I/O deps. Enforcement is the
  transitive dep graph (`cargo tree --edges normal` denylist),
  not a `Cargo.toml` regex. A `Cargo.toml`-only grep misses the
  common failure mode (a serialisation crate pulls an I/O dep
  through a feature flag).
- **R2** — The IR is versioned; consumers handshake. Adding a
  variant is a minor bump; removing or re-shaping is a major
  bump with a 12-month deprecation window. The dual-field
  tolerance pattern (V5 chart migration) is the canonical way
  to evolve a variant: accept both old and new field names for
  one release, default-write the new, drop the old after the
  window.
- **R3** — The React app never knows what the domain is.
  Enforcement is **structural**, not a fixed denylist. A
  per-crate `words.txt` allowlist is the contract; any string
  literal or identifier outside the allowlist (excluding
  comments, tests, fixtures) fails the build. A keyword
  denylist runs alongside as defence-in-depth.
- **R4** — Bindings resolve server-side; the client never sees
  expressions. The grammar lives in one place
  (`starter-ui-bindings`); the renderer is dumb projection.
- **R5** — One action endpoint, discriminated response.
  `form_errors` is renamed to `diagnostics` at the wire; no
  back-compat shim (D1).
- **R6** — Tables are queries, not row lists. RSQL grammar via
  the typed `rsql()` builder.
- **R7** — `custom` is the escape hatch, not a feature. Capability
  handshake threat model: `renderer_id` is public, `custom.props`
  authorisation runs at the handler / resolve boundary, capability
  filter is vocabulary not auth.
- **R8** — Size and DoS limits are enforced server-side. The
  evidence column in the SCOPE R8 table marks each limit as
  `Inherited / unmeasured`, `Inherited`, or `Reused`; first
  consumer-hit triggers measurement, not reflexive widening.
- **R9** — No client-side business logic. Optimistic hints exist
  for UX latency; the authoritative response is always the
  server's.

## Constraints

- **No I/O in `starter-ui-ir`** — dep-tree CI gate denylists
  `axum, axum-core, reqwest, hyper, tokio, tokio-util, tower,
  tower-http, h2, http-body`. A diff in the denylist means
  either the baseline updates (separate reviewed commit) or
  the change is rolled back.
- **No `starter-ui-bindings` dep from `starter-ui-builder`** —
  builder strings get resolve-time errors, not compile-time.
  Compile-time validation would require bindings to depend on
  bindings (defeating the split) and on a per-consumer
  `EntityGraph` shape (impossible without consumer-specific
  generics). The trade is intentional.
- **No feature flag on `starter-server`** — Cargo features
  can't prevent transitive workspace compilation. Consumer
  opts in via their own `Cargo.toml` dependency on
  `starter-sdui-routes` (M6).
- **`Renderer.tsx` ≤ 800 lines TSX, single file.** Size-budget
  CI gate.
- **Components total ≤ 4000 lines red line** across all
  components combined; per-component files vary. The libraries
  themselves (tiptap, monaco-diff) do not count.
- **Render against `@nube/starter-ui-kit`**, not Rubix's UI
  (D2). No parallel UI primitive library.
- **`diagnostics`, not `form_errors`** — at the wire (D1).
  Parse error on `form_errors` payloads.

## Phasing

- **Phase 1** — `starter-ui-ir` port + JSON Schema artifact +
  D1 (diagnostics rename) + dep-tree CI gate.
- **Phase 2** — `starter-ui-bindings` port + `EntityGraph`
  trait per S-D1 + "one page, N targets" smoke.
- **Phase 3** — `starter-ui-builder` port + compile-time /
  resolve-time contract documented + "Builder DSL produces
  valid IR" smoke.
- **Phase 4** — `@nube/starter-sdui-react` port + 16 initial
  components + size-budget CI gate + capability handshake.
- **Phase 5** — `starter-sdui-routes` crate + three routes +
  `HandlerRegistry` + `QueryEngine` trait + R8 limits with
  stable `what:` tags + capability-handshake threat model
  documented.
- **REVIEW** — core surface frozen (Component IR, EntityGraph
  trait, builder DSL, Renderer dispatcher, registries, routes).
- **Phase 6** — remaining IR components + streaming
  subscriptions + `$page` chart_range round-trip.
- **Phase 7** — `custom` escape-hatch wiring (registry +
  capability filter + fallback stub).
- **Phase 8** — optimistic action hints + diagnostics response
  + DoS limits enforcement + falsification suite.
- **Smoke gate** — ten SCOPE smokes + structural allowlist
  check + size-budget gate + dep-tree gate all green.

## Deliverables

- Three new workspace crates: `starter-ui-ir`,
  `starter-ui-bindings`, `starter-ui-builder` (each SemVer
  0.1.0).
- One new workspace crate: `starter-sdui-routes` (SemVer 0.1.0)
  — opt-in, not depended on by `starter-server`.
- One new npm workspace package: `@nube/starter-sdui-react`
  (SemVer 0.1.0).
- Three `words.txt` allowlist files (`crates/starter-ui-ir/`,
  `crates/starter-ui-bindings/`,
  `packages/starter-sdui-react/`).
- `DOCS/frontend/sdui/DIVERGENCE.md` populated with D1–D5
  through the phases.
- Ten SCOPE smoke tests passing in CI plus the structural
  allowlist check, the size-budget CI gate, the dep-tree CI
  gate, and the falsification suite.
- A worked example page authored from `main.rs` using the
  builder DSL that renders end-to-end through the Renderer
  against a fixture entity graph.

## Open questions (resolve in stage 1)

The source SCOPE flags five open questions; the runner pins
them in stage 1 before any code lands. Bias for each below.

1. **S-D1 — Where the `EntityGraph` trait lives.** Bias:
   **`starter-ui-bindings`**. Trait in `starter-ui-bindings`;
   consumers implement against their own graph. Promotion to
   `starter-spi` is mechanical (and signals a starter-wide
   seam) if and when a second consumer wants the trait —
   demotion isn't. Defer promotion until the second consumer
   exists.
2. **S-D2 — RSQL query engine: default or BYO?** Bias:
   **trait + in-memory reference impl in v1**. Production
   consumers wire their own backend. Porting Rubix's `query`
   crate is a separate project. The reference impl is enough
   for examples and the falsification suite.
3. **S-D3 — Visual page editor (drag-drop).** Bias:
   **defer**. Lands as a starter-extension (separate repo,
   MF bundle) when a consumer demands it. Builder DSL + AI
   authoring cover v1.
4. **S-D4 — `oneOf` server-resolved sub-form helper
   placement.** Bias: **renderer side**. The server emits
   the active variant; the renderer renders it like any
   other sub-form. Keeps the builder DSL surface narrow.
5. **S-D5 — Stream sentinel naming.** Bias: **inherit Rubix
   verbatim**. `{ type: stream_end, channel, reason: done |
   error | timeout | gone }`. No reason to bikeshed; pinning
   here.

Record decisions in **`DOCS/frontend/sdui/SCOPE.md`** under a
new `## Decisions` section before stage 3 (Phase 1) begins.
Do not edit any other SCOPE in this stage.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- **Dep-tree CI gate** — `cargo tree -p starter-ui-ir --edges
  normal` snapshot test fails the build if any of `axum,
  axum-core, reqwest, hyper, tokio, tokio-util, tower,
  tower-http, h2, http-body` appears. The transitive graph is
  the contract.
- **Structural allowlist check** — the CI script scans
  `crates/starter-ui-ir/src/`,
  `crates/starter-ui-bindings/src/`, and
  `packages/starter-sdui-react/src/` for any string literal or
  identifier (in source, not comments / tests / fixtures) that
  is not present in the per-crate `words.txt` allowlist.
  Allowlist additions require one sentence in the PR
  description naming the framework concept the word
  represents. "Convenience" is not a framework concept.
- **Keyword denylist (defence-in-depth)** — a secondary check
  for the obvious cases (`building|device|alarm|...`); the
  allowlist is the real contract, the denylist is the
  tripwire.
- **Size-budget CI gate** — `Renderer.tsx` ≤ 800 lines TSX
  (single file). Total component implementations ≤ 4000 lines
  red line across all components combined. The libraries
  themselves (tiptap, monaco-diff) do not count toward the
  budget; only the IR-adapter wrappers do.
- **No `starter-server` → `starter-sdui-routes` dep** —
  `cargo tree -p starter-server --edges normal` snapshot test
  fails if `starter-sdui-routes` appears. The consumer-opt-in
  claim from M6 is structural.
- **`diagnostics` not `form_errors`** — a wire-level parse
  test asserts a `form_errors` payload is rejected with a
  structured error naming the field.
- **Capability mismatch refuses to render** — a smoke loads a
  V+1 tree against a V client and asserts `<SduiPage>` shows
  the mismatch banner; the tree never reaches the dispatcher.
- **Custom fallback works** — a smoke renders `{ type:
  custom, renderer_id: unknown.id }` and asserts the stub
  renders, the rest of the tree renders normally, and a
  structured warning is logged.
- **DoS limit `what:` tags** — each of the seven R8 limits has
  a fixture that violates it; each returns 413 with the
  expected stable tag. Pins the enforcement, not the value.
- **Falsification suite** — three fixture pages (CRUD device
  list, PR review card with diff + inline actions, scope
  board with state badges + live updates) render end-to-end
  through one renderer with zero domain-specific strings in
  the renderer crates (R3).
