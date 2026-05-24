# Workflow — rubix-frontend-surfaces

## Sequencing

17 stages across five phases. Order: A (authz triage + nav) → B (flow browser) → C (CH + insights admin) → D (chrome polish) → E (closing docs + PR). Four REVIEW gates.

This is a **consumption job**. The starter UI packages are stable and complete; the work is mounting them into routes, wiring rubix-specific overrides through the existing seams, and adding one rubix-side warehouse admin panel set. Resist the urge to grow starter packages — if a primitive is missing, raise BLOCKED and file an upstream issue; don't hand-roll a workaround.

## Per-stage discipline

### Phase A — authz admin completion

Two stages. A.1 is a triage walk-through; A.2 is the nav + e2e.

1. **A.1 starts with manual exploration.** Spin up `make start`, log in, walk through every tab in `/admin/access`. The closing-doc evidence in PR #35 says the route exists; this job confirms each tab actually works. Classify failures as (a)/(b)/(c) per SCOPE; fix only (c) (i18n / styling / wiring) in this stage.
2. **Don't fix backend issues in this job.** If a tab calls a backend endpoint that returns 500, that's class (a) — track as a follow-up, don't patch.
3. **The left-nav admin section uses existing starter-ui-kit primitives.** No new nav component design. If the kit's nav doesn't support nested sections, raise BLOCKED.
4. `pnpm --filter @nube/rubix-frontend typecheck + test + e2e` green per stage.

### Phase B — flow browser

Four stages. The core integration risk: where does the flow body come from?

1. **B.1 first checks the data source.** Read `rubix-client-ts/src/endpoints/flow_ops.ts` end-to-end. If `flow-ops.list` returns only metadata, look for `flow-ops.get(id)` or any other admin route that exposes the body. If none, BLOCKED.
2. **B.2 is the rubix-side override layer.** The `@nube/starter-ui-flow` package is generic; rubix overrides the `ai-agent` node component to render `skill_hint` + `allowed_tools[]`. The override registers via `NodeKindRegistry::register(...)` — this is the contract seam. Do not modify the upstream package.
3. **The flow body is parsed client-side.** The `yaml` npm package handles the parse. Confirm at B.1 that `yaml` is reachable (likely already a transitive dep via vite or starter-ui-kit). If not, add it as a direct dep of `rubix/frontend` only — not as a starter-package dep.
4. **`readOnly={true}` everywhere.** Authoring flows in the UI is firmly out of scope. The canvas exposes the prop; consume it.
5. **The smoke test asserts structural correctness, not visual.** "Canvas renders ≥ 1 node labelled ai-agent" is the bar; pixel-perfect rendering is a follow-up.

### Phase C — ClickHouse + insights admin

Four stages. The largest phase by file count.

1. **C.1 confirms backend coverage before any UI work.** Grep `rubix-agent/src/routes/` for `clickhouse` and `insights` admin endpoints. If any of the 8 hook-targeted endpoints are missing, BLOCKED — fix lands in a separate rubix-agent job. Working around server-side gaps in the frontend is forbidden (it'd just produce a confusing UI).
2. **C.2 lands hooks before panels.** Hooks + their sibling tests in one commit; panels in a separate commit that consumes them. The split makes each commit reviewable.
3. **C.3's four panel files are verb files.** Each ≤ 200 lines. If a panel grows beyond that, split (e.g. `rules-panel.tsx` + `rules-editor.tsx`).
4. **The tabbed shell mirrors `<AuthzAdmin>` exactly.** Same `Tabs` primitive, same tab-state preservation across route visits, same i18n shape. A future maintainer reading both should not have to re-learn the convention.
5. **Plain `<textarea>` is enough for SQL editing v1.** Resist the urge to integrate Monaco/CodeMirror here — it's a polish follow-up; this job is about getting the surface wired.

### Phase D — chrome polish

Three stages. Looks small; carries the user-facing impact.

1. **The top-header tenant indicator is display-only.** Tenant switching needs backend session re-scoping that doesn't exist; this job ships the visible affordance + an honest tooltip about the limitation.
2. **The toast listener uses TanStack Query's `QueryCache` `onError`** — a known pattern. Not a global `window.onerror` handler.
3. **Theme toggle wires to existing CSS variables.** The frontend already has `theme.css` + `tokens.css` per the earlier inventory; toggle just flips a class on `<html>`. Don't ship a full theme editor (`starter-ui-kit` already has one; don't open it in this job).
4. **Loading skeletons match the eventual layout shape.** A flat grey bar where a row table will be doesn't help users; mirror the shape (3 rows × N columns of grey blocks).

### Phase E — closing

One stage. Three artifacts (design doc extension, session note, THIN-SLICE update) + the PR.

1. **The session note lists every BLOCKED follow-up.** This is the operator's "what's still broken backend-side that we need to file" handoff. Be specific.
2. **The PR title is the elevator pitch.** `feat(rubix-frontend): consume starter UI packages + ClickHouse admin surfaces`. The body summarises per phase.

## Anti-patterns specific to this job

- **Don't modify any existing starter UI package.** Use the registration seams (`NodeKindRegistry::register`, custom i18n via `useAuthzMessagesFromIntl`, etc.). If a seam is missing, file an upstream issue and BLOCK.
- **Don't author new upstream starter packages.** ClickHouse admin is rubix-specific; it lives in rubix.
- **Don't hand-roll primitives starter-ui-kit already exposes.** Toast, Skeleton, EmptyState, Tabs — all should come from the kit. If you're typing the implementation of one of those, stop.
- **Don't add backend code in this job.** If a REST endpoint is missing, BLOCKED. Working around it in the frontend with mocked data is forbidden.
- **Don't ship a fake "live" panel.** If the `DecisionsPanel` or `InsightsPanel` can't reach a real endpoint, surface an honest "endpoint not wired" message, don't fake data.
- **Don't add tenant switching.** Out of scope; display-only.
- **Don't open the theme editor in the top-header toggle.** Just flip light/dark.
- **Don't list paths with brace expansion in handovers.**
- **Don't list a path under Done that the stage didn't modify.**
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Four gates. Each commits and pushes the stages that led to it; the gate itself commits nothing.

Handover at each gate must include:

- One-line title per commit.
- `pnpm typecheck + test + e2e` counts (e2e against a running backend per `make start`).
- For A: list of broken-tab follow-ups split by class (a)/(b)/(c).
- For B: confirmation that the canvas renders the bundled flow YAMLs without error.
- For C: list of missing backend endpoints if any were BLOCKED, otherwise per-tab smoke evidence.
- For D: one operator-runnable manual flow exercising the full nav.
- Any deviation from SCOPE.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-frontend-surfaces`.

REVIEW gate stages mark `git` as `skipped — gate-only`. A.1 marks `git` as `skipped — analysis only` if no (c) fixes landed. C.1 marks `git` as `skipped — analysis only` if every endpoint exists. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. TS ≤ 200 lines hard.
- Source comments link `docs/design/<area>/README.md` only.
- No phasing markers in code.
- Upstream-first (R2). Zero upstream changes in this job; raise BLOCKED if one is needed.
- Errors typed. `UseQueryResult<T, RubixError>` everywhere.
- Catalogue files are the source of truth for MessageKeys.
- Tests live with the code in the same commit.
- No `any`. Strict TS.
- Comments explain *why*, not *what*. No emojis.

## References

- `packages/starter-ui-authz/` — admin panels for Phase A.
- `packages/starter-ui-flow/` — canvas + node registry for Phase B.
- `packages/starter-ui-kit/` — Toast, Skeleton, EmptyState, Tabs, Nav primitives.
- `rubix/packages/rubix-client-react/src/hooks/` — existing hooks; extended in B + C.
- `rubix/frontend/src/routes/admin/access.tsx` — model for new admin routes.
- `rubix/frontend/src/components/top-header.tsx` — extended in D.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — backend surface the frontend consumes.
- `rubix/Makefile` — `make start` is the operator-runnable smoke flow.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
