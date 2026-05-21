# Insights Mockup — Progress

Tracking implementation of [INSIGHTS-MOCKUP.md](./INSIGHTS-MOCKUP.md).

## Stage checklist

- [x] **S0 — Survey & scaffold**
  - Read spec, survey repo layout. Create fixtures dir with placeholder + this progress doc.
- [ ] **S1 — Fixtures: schema + seed data (Phase 1 data)**
  - Author `fixtures/insights/{rules,verdicts,pipelines,coverage,tags-index}.json` covering IoT, Energy, Bills scenarios per spec §Fixtures. Write `fixtures/insights/README.md`.
- [ ] **S2 — Backend `insights_mock.rs` (Phase 1 backend)**
  - New module wired into `server.rs` exposing the 9 REST routes in spec §Backend. `RwLock<InsightsFixtures>` in app state, fixture loader on startup. Smoke tests via `tests/`.
- [ ] **S3 — Frontend Phase 1 surfaces**
  - Sidebar Insights section. `RulesList.tsx` + `VerdictsView.tsx` (list + detail) reading the REST. React-query hooks. Routes wired in `app.tsx`.
- [ ] **S4 — Print stylesheet (I4)**
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

(empty)

## Run log

### S0 — 2026-05-22
- Repo on branch `mock-rules`. Surveyed `examples/flow-agent/src/` and `frontend/src/pages/`.
- Created `fixtures/insights/` placeholder and this progress doc.
- Commit + push.
