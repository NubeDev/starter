# Workflow — rubix-dashboards-goal-1

## Sequencing

16 stages across five phases (one with grouped sub-stages). Order: A (substrate — 01-storage + 02-bindings, parallelisable within the phase) → B (host glue 03) → C (tools 04, five sub-stages) → D (frontend renderer 05 + AI builder + adoption 06) → E (closing). Five REVIEW gates.

This job is **infrastructure-already-built**, like extensions-wire was. Every piece of upstream substrate exists; the work is filling specifically-itemised gaps and wiring. Resist the urge to redesign anything in `starter-ui-ir` / `starter-ui-bindings` / `starter-sdui-routes` — read the scope files, fill the gaps, ship.

## Per-phase discipline

### Phase A — substrate (01 + 02)

Two stages, both upstream-anchored.

1. **A.1 mirrors PR #32's `flows_definitions` shape exactly.** Reviewers infer the dashboards table from the flows table. Don't invent new column names or new indexing — copy the pattern.
2. **A.2's seven commits land in sequence and each must compile + test green standalone.** G1 splits across two crates (ir adds the trait, bindings dispatches through it) — those are two adjacent commits, not one. Bisect targets every commit, so each one must build.
3. **The six bindings gaps are < ~200 LOC each per 02-bindings-gaps.md.** Each commit's diff stays small. If a single gap balloons past 300 lines, something's wrong — re-read the scope file.
4. **Test live with each commit (R6).** `starter-ui-bindings/tests/` already has the harness; extend it per gap.
5. `cargo test -p starter-ui-bindings -p starter-ui-ir` green after every A.2 commit.

### Phase B — host glue (03)

Two stages. The integration core.

1. **Four trait impls, four files.** Don't merge into one — verb-per-file (R1).
2. **`EntityGraph` reads slots via the flow engine seam.** Don't re-implement slot reading; the engine exposes it. Read the resolver / propagator source to find the right entry point.
3. **`QueryEngine` routes by query target.** ClickHouse queries via the existing `ChClient` (no new connection management). PG queries via the existing pool. If a query category doesn't fit either, surface `rubix.dashboard.query.unsupported` Diagnostic — don't invent storage backends.
4. **`HandlerRegistry`'s `action` dispatch is every-action-is-a-tool-call.** Per 03-host-glue.md. No bespoke action handlers in the impl — the tool registry IS the handler registry.
5. **B.2's merge with `main.rs` is the one known overlap with the in-flight `rubix-flow-live-tick-demo` job.** Adjacent additions, not same lines. Whichever PR merges second rebases cleanly. If a real conflict surfaces during stage B.2, raise it in the handover and resolve in the worktree (preserve both router merges).

### Phase C — tools (04, five sub-stages)

Filling seven verb stubs. The sub-stage split is by reversibility + read/write semantics: reads → simple writes → trickier writes → runtime-write → integration.

1. **Each verb gets its DTO in `rubix-spi`, its body in `rubix-tools`, its MessageKeys in catalogues, all in the same commit.** R5 + R6.
2. **Reversibility is via `starter-undo`'s dispatch** (mirrors goals-2-4-3's flow_ops). Don't invent a new undo mechanism.
3. **`update`'s optimistic concurrency uses `expected_revision_id`** — same pattern flow_ops would use. Conflict returns a typed `rubix.dashboard.update.conflict` Diagnostic, not a 409 with no detail.
4. **`delete` refuses on `created_by="system"`** — system-protection. Document the rationale in the design doc.
5. **`page_set` is special — runtime write, not a revision.** Document loudly that it's not reversible via `undo.last`. Operator reverts by setting the slot back.
6. **C.5 wires the dispatch + integration test.** The tools auto-surface as MCP tools per R7 — no MCP wiring code per the existing flow_ops pattern.

### Phase D — frontend renderer + AI builder (05 + 06)

Two big stages. D.1 lands the upstream package; D.2 consumes it in rubix.

1. **D.1's `@nube/starter-ui-sdui-react` is a brand-new starter package.** Use `starter-ui-flow`'s shape as the template — same `package.json` keys, same `tsconfig.json` inheritance, same `vitest.config.ts`, same README convention.
2. **Renderer-per-variant.** Fourteen renderer files (one per IR variant). Each ≤ 150 lines. Each consumes `starter-ui-kit` primitives — never hand-roll a primitive.
3. **Transport seam is the key abstraction.** The package has no HTTP code; `HttpSduiTransport` is the concrete impl. Test the package against a `MockTransport` in vitest; test the http impl separately against MSW.
4. **D.2 is the rubix consumer.** Three concrete pieces: the flow rewrite, the bundled page, the routes + provider + e2e. Land them together so the Playwright spec asserts the full chain.
5. **Replace `dashboard.tsx` with a redirect.** Keep the user-visible URL `/dashboard` working — redirect to `/dashboards/disk-overview` so existing bookmarks survive.
6. **The bundled `disk-overview.json` page is the worked example** the design doc cites. Real bindings (`{{$target.percent_used}}`), real query (`useDiskUsage`-equivalent), real chart, real `page_set` action. Not a fake.
7. `pnpm --filter @nube/rubix-frontend typecheck + test + e2e` green at D.2 close.

### Phase E — closing

One stage. Promote scope to design docs.

1. **Each promoted file is present-tense.** "The dashboards table has columns X, Y, Z" — NOT "we will add a dashboards table." The scope file's "decision" sections become the design doc's body.
2. **The scope file gets replaced with a redirect** or deleted. Don't leave stale scope files alongside fresh design docs.
3. **07-fetch-plan.md stays.** v2 hand-off; the scope folder still exists for unresolved items.
4. **08-open-questions.md gets emptied** as the codeless agent folds answers into the design docs. Any unanswered question stays in the file as a future-decision marker.

## Anti-patterns specific to this job

- **Don't redesign `starter-ui-ir` or `starter-ui-bindings`.** The substrate is stable. Fix the six itemised gaps in 02-bindings-gaps.md; don't expand scope.
- **Don't hand-roll renderer primitives.** Use `starter-ui-kit`.
- **Don't write a second IR.** Per the README non-goal.
- **Don't add a client-side template language.** Bindings live and die on the server.
- **Don't per-tenant-partition pages in v1.** `tenant_id` column + filter on principal is enough.
- **Don't add `Custom` renderer extension surface yet.** Phase 3 widget marketplace is out of scope.
- **Don't add `FetchPlan` to anything in this job.** v2; 07-fetch-plan.md stays unpromoted.
- **Don't `page_set` reversible.** Document the choice; runtime slot writes are not revisions.
- **Don't list paths with brace expansion in handovers.** Trips diff-verify.
- **Don't list a path under Done that the stage didn't modify.** Same trap.
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Five gates: A↔B, B↔C, C↔D, D↔E, plus the final pre-PR gate inside E. Each commits and pushes the stage(s) that led to it; the gate itself commits nothing.

Each gate's handover must include:

- One-line title per commit.
- `cargo test` + `pnpm typecheck/test/e2e` summary.
- For A: a one-line demonstration that a non-text-widget template now resolves (e.g. a `chart.node_id: "{{$target}}"` example with input + output).
- For B: a curl manual flow demonstrating `/api/v1/ui/resolve` returns a `ComponentTree` for a seeded page row.
- For C: a curl manual flow demonstrating the seven-verb roundtrip (create → list → get → update → undo → delete → duplicate).
- For D: a browser manual flow demonstrating `/dashboards/disk-overview` renders live kpi + chart + responds to a `page_set` action.
- Any deviation from SCOPE.
- Open Questions evidence where the stage answered one.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-dashboards-goal-1`.

REVIEW gate stages mark `git` as `skipped — gate-only`. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. Rust ≤ 400 lines hard, TS ≤ 200 lines hard.
- Code comments link `docs/design/sdui/<area>/README.md` only — never the scope files.
- No phasing markers in code.
- Upstream-first (R2). A.2 bindings + D.1 sdui-react land before rubix consumes.
- Tool outputs are `Diagnostic` + structured data.
- Catalogue files are source of truth for MessageKeys.
- Tests live with the code in the same commit.
- R7 — every tool auto-surfaces as MCP via the registry.
- R10 — reverse-DNS ids.
- R12 — MCP resource URIs distinct from SDUI page ids.
- No emojis. Comments explain *why*, not *what*.

## References

- `rubix/docs/scope/dashboards/README.md` — master index for the entire job.
- `rubix/docs/scope/dashboards/01-storage.md` … `06-ai-builder.md` — per-phase authoritative briefs.
- `rubix/docs/scope/dashboards/08-open-questions.md` — defaults to fold into design docs.
- `crates/starter-ui-ir/`, `crates/starter-ui-bindings/`, `crates/starter-sdui-routes/`, `crates/starter-ui-builder/`, `crates/starter-ui-theme/`, `crates/starter-tags/` — substrate.
- `rubix/crates/rubix-tools/src/dashboard/` — verb stubs to fill.
- `rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md` — AI skill file.
- `rubix/crates/rubix-flows/flows/dashboard-assistant.yaml` — stub to rewrite.
- `packages/starter-ui-flow/` — template for the new `starter-ui-sdui-react` package shape.
- `rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md` — current handover + codeless runbook.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
