## Done

- §B3 landed: `src/curation/data-sources.ts` curation table + `<DataSourceField>` / `<CatalogueProvider>` rubix-agnostic seam; build-puck-config consults DATA_SOURCES before the default type dispatch and emits Puck `custom` fields tagged with `catalogueKind`.
- PuckBuilder gained an optional `catalogue` prop that wraps the canvas in the provider.
- Harness wires a mock catalogue (5 kinds); rubix-frontend edit route wires a real one (`/api/v1/tools`, `rubix.tenant.list`, static unit-symbol suggestions, $page key list). Analytics-template lookup rejects until the list verb lands → operator gets free-text fallback per scope §B3 "degrade to free-text with an inline warning".
- Test file expanded (per-kind coverage + per-tuple custom-field assertion). Stale snapshot regenerated.
- README §B3 row flipped ⏳ → ✅; the duplicate B3 entry pulled from the "Next tasks" list.
- `pnpm --filter @nube/starter-ui-sdui-puck typecheck` and `test` green; `pnpm --filter @nube/rubix-frontend typecheck` green.
- Committed as `54c8039` on `codeless/puck-builder-finish`.

## Next

- Stage 2 (per the job-file workflow) — `§B6 runtime schema-hash banner` is next on the scope list; the README "Next tasks" section enumerates the remaining follow-ups (multi-tenant wiring, discard-bridge ref, placeholder coverage, stale render-chart test, scope 11).

## What you need to know

- The IR doesn't actually declare named types like `AnalyticsTemplateRef` / `ToolRef` — those are conceptual. v1 tuples are all top-level paths (`kpi.unit_symbol`, `kpi.source`, `sparkline.unit_symbol`, `action_widget.action_ref`, `drawer.open`). Nested array paths (`chart.sources[].name`) are deferred until B3 PR2 / union picker.
- `CatalogueKind` covers 5 kinds; `tenant` has no IR leaf wired yet — plumbing exists for B3 PR2. The test's per-kind coverage check intentionally skips `tenant`.
- The analytics-template list verb wasn't discovered (no `rubix.analytics.template.list` in the repo); the rubix-frontend kind throws and the picker degrades — replace with the real verb when it ships.
- `PuckFieldStub.custom` was extended with an optional `catalogueKind` tag so tests/devtools can identify a picker's binding without instantiating it. Puck ignores extra keys.
- `ChartSource` picker writes back `{ type: "analytics_template", name: <pick>, …prev }` — picking a non-analytics_template variant is not yet supported.

## Open questions

- Which verb actually lists analytics template stems? Scope §B3 names `rubix.analytics.list_templates` but the codebase only has `rubix.analytics.query` / `rubix.analytics.report`. The kind currently rejects; the next session may want to either add the verb or rename the kind.
