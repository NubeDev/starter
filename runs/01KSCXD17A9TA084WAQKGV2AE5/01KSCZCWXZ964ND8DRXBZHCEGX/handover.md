## Done

- Added rubix/frontend/src/components/admin/warehouse/ with five files: rules-panel.tsx, marts-panel.tsx, retention-panel.tsx, insights-panel.tsx, warehouse-admin.tsx (tabbed shell mirroring AuthzAdmin), plus index.ts barrel. Each panel is a verb file under 200 lines and consumes hooks from @nube/rubix-client-react.
- Wired /admin/warehouse route to mount <WarehouseAdmin> (replaced the Phase A stub).
- Added 37 admin.warehouse.* keys to rubix/frontend/src/i18n/en.json and rubix/frontend/src/i18n/es.json in the same commit.
- Exported the insights hooks family (useInsightsRulesList / *Create / *Enable / *Disable) from @nube/rubix-client-react/src/index.ts — they existed but were not re-exported (typecheck-blocker).
- pnpm --filter @nube/rubix-frontend typecheck and pnpm --filter @nube/rubix-client-react typecheck both green.
- Committed as 0282149: "stage 11: phase C.3 — warehouse admin panels — feat(rubix-frontend) warehouse admin panels".

## Next

- (none) — stage 11 closes; a fresh session picks up stage 12.

## What you need to know

- The job spec named the write hooks "useClickhouseRuleWrite" and "useClickhouseRetentionSet"; the actual exports in rubix-client-react are useRuleWrite and useRetentionSet (no useClickhouse* prefix). Used the real names.
- No rule.drop verb exists on the backend; rules-panel's "delete" implements the soft-delete spec by writing the DDL `-- soft-deleted via warehouse admin` via useRuleWrite. window.confirm gates it.
- Marts drop uses window.confirm with a hard "cannot be undone" copy (admin.warehouse.marts.confirmDrop, with {name} interpolation done in-component).
- Retention panel tracks per-row drafts in component state; Save button enables only when the draft differs from the current retention_days.
- The pre-existing top-level "warehouse.*" i18n keys (used by home-feature tile) were left intact; the new keys are namespaced under "admin.warehouse.*".

## Open questions

- (none)
