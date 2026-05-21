# Insights Mockup — Progress

Tracking implementation of [INSIGHTS-MOCKUP.md](./INSIGHTS-MOCKUP.md).

## Stage checklist

- [x] **S0 — Survey & scaffold**
  - Read spec, survey repo layout. Create fixtures dir with placeholder + this progress doc.
- [x] **S1 — Fixtures: schema + seed data (Phase 1 data)**
  - Author `fixtures/insights/{rules,verdicts,pipelines,coverage,tags-index}.json` covering IoT, Energy, Bills scenarios per spec §Fixtures. Write `fixtures/insights/README.md`.
- [x] **S2 — Backend `insights_mock.rs` (Phase 1 backend)**
  - New module wired into `server.rs` exposing the 9 REST routes in spec §Backend. `RwLock<InsightsFixtures>` in app state, fixture loader on startup. Smoke tests via `tests/`.
- [x] **S3 — Frontend Phase 1 surfaces**
  - Sidebar Insights section. `RulesList.tsx` + `VerdictsView.tsx` (list + detail) reading the REST. React-query hooks. Routes wired in `app.tsx`.
- [x] **S4 — Print stylesheet (I4)**
  - `@media print` CSS for verdict detail (and rule editor detail header). Hide chrome, table evidence rows, `@page` numbers, light-only.
- [x] **S5 — `RuleEditor.tsx` + dry-run (Phase 2 UI)**
  - Split-pane page with editor (textarea, see D-S5-1), schema panel, dry-run pane. No agent tools yet — just human edit + POST.
- [x] **S6 — Agent tools: rule.{list,read,propose,apply,dry-run} (Phase 2 agent)**
  - Register tools with the real `AiRunner`. `propose` returns proposal object (no write); `apply` commits. Diff card in chat (UI half deferred to S9 polish — see D-S6-2).
- [x] **S7 — Verdict agent tools (Phase 3)**
  - `verdict.query`, `verdict.explain`. Tool defs synthesised in same pass as S6 (rule + verdict + pipeline share one bridge — see D-S6-1). UI agent dock deferred to S9 polish.
- [ ] **S8 — PipelineCanvas + pipeline tools (Phase 4)**
  - `PipelineCanvas.tsx` wrapping `starter-ui-flow`. Tools `pipeline.read`, `pipeline.propose-edit`, `pipeline.apply-edit`. Diff overlay.
- [ ] **S9 — Polish & acceptance checklist**
  - Run through spec §Acceptance checklist. Theme-print sanity, retro-correction flag visibility, audit-trail check.

Each stage commits + pushes at the end.

## Design decisions

- **D-S1-1** — Verdict shape in `verdicts.json` mirrors `Verdict` from
  SCOPE.md §475 (rule_id, at, tz, window, severity, coverage, tags,
  summary, evidence, correlation_id). `coverage` carries the
  `raw`/`effective`/`quality_flags` split so the UI never has to fake
  that structure. AI verdicts get an extra top-level
  `ai_explanation` string and model/cost rows in `evidence`.
- **D-S1-2** — Retroactive-correction is encoded by attaching the
  `starter.quality.retroactive-correction@1` flag to the verdict's
  `coverage.quality_flags` (per SCOPE D5) plus a `supersedes` field
  in the evidence row pointing to the original verdict id. This lets
  S4's print stylesheet show the flag without a side table.
- **D-S1-3** — `pipelines.json` graphs use `{nodes:[{id,kind,x,y,rule_id?}],
  edges:[{from,to,type}]}` where `type ∈ {Dataset, Verdict, Frame}`
  per spec §PipelineCanvas. The canvas layer (S8) will translate to
  the `starter-ui-flow` node model.
- **D-S2-1** — `insights_mock.rs` keeps rows as raw `serde_json::Value`
  (not typed Rust structs). Reason: the fixture JSON shapes are
  load-bearing (I2), not the Rust types; staying schema-less means
  we don't need to keep two definitions in sync. The eventual
  `starter-insights` crate will introduce typed `Verdict`/`Rule`
  structs and this module will be deleted (I5).
- **D-S2-2** — Insights router exposed as its own
  `router<S>(state) -> Router<S>` with `with_state(state)` baked in,
  then merged in `server::build`. Avoids widening `RestState` for a
  surface we'll delete.
- **D-S2-3** — `POST /api/insights/pipelines` covers both create and
  graph-update by id (spec lists POST for both). Halves the
  surface area without losing capability.
- **D-S2-4** — Fixture loader degrades to an empty store with a
  warn-level log if files are missing. Keeps `cargo run -p flow-agent`
  bootable from any CWD even when the bin is launched outside the
  workspace.
- **D-S3-1** — `VerdictsView.tsx` handles both list and detail
  routes via a single component switching on `:id`. Keeps the
  page count down and matches the spec's "filterable list + detail"
  framing. Print-only header/footer markup ships now so S4 can
  layer the `@media print` CSS without re-touching the page.
- **D-S6-1** — One bridge pass covers all three insights tool
  families (`rule.*`, `verdict.*`, `pipeline.*`). Synthesis lives in
  `agent_bridge::synthesize_insights_tools`; dispatch lives in
  `dispatch_insights_tool`. Cheaper than three iterations of the
  same scaffolding; the verdict + pipeline backends (S7 + S8 in
  the original plan) collapse to "register the tool name + write
  the match arm." So S7 ships with S6; the remaining S7/S8 frontend
  work (agent dock UI) is folded into S9 polish.
- **D-S6-2** — Tool wildcards: agents opt in via `insights:*`,
  `insights:rule.*`, `insights:verdict.*`, `insights:pipeline.*`, or
  individual tool names. Matches the `flow:*` convention already in
  the runtime so operators don't learn a new syntax.
- **D-S6-3** — `propose` is non-mutating by design: returns a JSON
  proposal blob the UI / operator inspects before calling `apply`.
  Spec §Agent tools: "the agent never silently mutates." Verified
  by `propose_does_not_mutate_apply_does` test.
- **D-S6-4** — `AiRuntime` gains an optional `insights:
  Option<InsightsState>` field via a builder `with_insights(state)`.
  Keeping it `Option` means existing call-sites (tests, alternate
  bootstraps) don't have to pass an insights handle they don't
  need. `synthesize_insights_tools` no-ops when `None`.
- **D-S5-1** — `RuleEditor.tsx` uses a plain `<Textarea>` instead of
  Monaco/CodeMirror (MD1 in spec). Rationale: Monaco pulls in
  ~3 MB of language workers; CodeMirror still adds ~150 KB. For a
  Phase 2 mock-up whose code may be rewritten when starter-insights
  lands (I5), the textarea is honest enough. The acceptance
  checklist doesn't require syntax highlighting. Reopen MD1 if a
  real user demo asks for it.
- **D-S5-2** — Dry-run sends `{ body }` in the POST payload even
  though the mock ignores the input (D-S2-3); the eventual real
  backend will need the working-copy body to evaluate, so wiring
  it now keeps the agent-tool contract (S6) shaped right.
- **D-S4-1** — Print CSS lives in `globals.css` (not lifted into
  `starter-ui-kit`) — defers MD4 until Phase 1 ships and other
  detail pages prove the layout is reusable. Locality + zero
  cross-package churn is the right trade now.
- **D-S4-2** — `@page :first { margin-top: 12mm }` and a single A4
  size keeps the first-page header tight without per-page CSS for
  evidence-heavy verdicts. Tested mentally against the existing
  fixture rows; any verdict in the seed set fits on one page.
- **D-S3-2** — Sidebar "Insights" entry sits *above* Skills/Settings
  (per spec §Sidebar additions: above Pages — but flow-agent's
  Pages sits high in the nav, so above Skills is the closest
  practical match without reordering existing items).

## Run log

### S0 — 2026-05-22
- Repo on branch `mock-rules`. Surveyed `examples/flow-agent/src/` and `frontend/src/pages/`.
- Created `fixtures/insights/` placeholder and this progress doc.
- Commit + push.

### S1 — 2026-05-22
- Wrote `rules.json` (7 rules across iot/energy/finance), `verdicts.json`
  (9 verdicts including 1 Error, 1 Critical, 1 retroactive-correction, 1
  partial-onboarding, 1 AI-judge), `pipelines.json` (3 graphs),
  `coverage.json`, `tags-index.json`, and `README.md`.
- Validated all JSON with `python3 -m json.tool`. No Rust/TS code yet,
  so no build to run.
- Commit + push.

### S6 / S7 (folded) — 2026-05-22
- `AiRuntime` gains optional `insights: InsightsState` via
  `with_insights(state)`.
- `agent_bridge`:
  - new `synthesize_insights_tools(agent_tools)` returns 5 rule +
    2 verdict + 3 pipeline `ToolDef`s, filtered by wildcards.
  - new `dispatch_insights_tool(tu)` handles all 10 tool calls;
    `propose` returns a `needs_approval: true` proposal, `apply`
    writes through `InsightsFixtures::persist_array`.
  - `drive_chat` now appends insights tools to the per-turn tool
    list; `dispatch_tool_use` routes `insights:*` calls to the new
    dispatcher and emits the same `tool-call` / `tool-result` SSE
    frame pair as flow tools.
- `server::build` constructs the runtime with `with_insights(...)`
  after loading fixtures so every agent on the system can opt in.
- New `tests/insights_agent_tools.rs`: 3 tests (synth wildcard, read
  round-trip, propose-doesn't-mutate-apply-does).
- `cargo build -p flow-agent` ✓; `cargo test -p flow-agent` ✓
  (9 tests passing: 1 existing + 5 backend + 3 agent-tool).
- Commit + push.

### S5 — 2026-05-22
- New page `frontend/src/pages/RuleEditor.tsx`: split-pane with body
  (textarea) + schema tabs on the left, dry-run pane on the right;
  summary/tags inputs; Save (PATCH) and Dry-run (POST) actions.
- Extended `api.insights.*` with `updateRule` and `dryRunRule`.
- `RulesList` ID column linkified to `/insights/rules/:id`; new route
  added in `app.tsx`.
- `pnpm typecheck` ✓; `pnpm build` ✓.
- Commit + push.

### S4 — 2026-05-22
- Appended `@media print` block to `frontend/src/globals.css`:
  pinned light palette inside `.dark` so theme can't bleed into PDFs
  (acceptance checklist item), hid sidebar/header/footer/buttons,
  promoted `print:block` elements, flattened badges to greyscale
  outlines, set `@page A4 16mm/14mm`, tightened verdict typography.
- The `verdict-print` class was already added to the verdict article
  in S3, so no page-component changes needed.
- `pnpm typecheck` ✓; `pnpm build` ✓ (CSS bundle 170→172 KB, +2 KB).
- Commit + push.

### S3 — 2026-05-22
- Extended `frontend/src/lib/api.ts` with Insights types
  (`InsightsRule`, `InsightsVerdict`, `InsightsPipeline`, `InsightsSeverity`,
  `InsightsCoverage`, `InsightsVerdictFilter`) and `api.insights.*` client.
- New pages: `RulesList.tsx` (filter + table) and `VerdictsView.tsx`
  (list + detail with severity badges, coverage %, evidence table,
  quality flags, AI explanation, print actions/header/footer scaffolding).
- Sidebar: added Insights group (Rules + Verdicts) in `Shell.tsx`,
  title/activeUrl handlers updated, routes wired in `app.tsx`.
- `pnpm typecheck` ✓; `pnpm build` ✓ (frontend bundle clean).
- Commit + push.

### S2 — 2026-05-22
- Added `src/insights_mock.rs` (rules, verdicts, pipelines CRUD + dry-run +
  coverage/tags helpers) with `tokio::sync::RwLock<InsightsFixtures>` and
  pretty-printed write-back to disk.
- Wired into `server.rs` via `merge_router(insights_router(...))`. Default
  fixtures dir resolves via `CARGO_MANIFEST_DIR` with
  `INSIGHTS_FIXTURES_DIR` override.
- New `tests/insights_mock.rs`: 5 tests (list rules, get + 404, filter
  verdicts, dry-run, upsert pipeline round-trips to disk via tempdir).
- `cargo build -p flow-agent` ✓; `cargo test -p flow-agent` ✓ (6 tests
  passing total: 1 existing + 5 new). 0 boundary-check script in repo.
- Commit + push.
