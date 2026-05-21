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
- [ ] **S5 — `RuleEditor.tsx` + dry-run (Phase 2 UI)**
  - Split-pane page with Monaco editor (MD1), schema panel, dry-run pane. No agent tools yet — just human edit + POST.
- [ ] **S6 — Agent tools: rule.{list,read,propose,apply,dry-run} (Phase 2 agent)**
  - Register tools with the real `AiRunner`. `propose` returns proposal object (no write); `apply` commits. Diff card in chat.
- [ ] **S7 — Verdict agent tools (Phase 3)**
  - `verdict.query`, `verdict.explain`. Agent dock on VerdictsView.
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
