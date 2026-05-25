## Done

- added rubix/frontend/e2e/flow-live-tick.spec.ts covering live SSE tick, hot-edit step=10, and reload-persistence against the bundled com.rubix.tick-counter flow
- verified spec parses via `pnpm exec playwright test --list flow-live-tick.spec.ts` (1 test listed)
- committed as cb934fa "phase E.4 — playwright live-tick spec"

## Next

- stage 15 of 16: next session picks up the next phase per the job plan

## What you need to know

- spec assumes the live backend has `mani run demo` style seed (operator op@example.com / rubix-dev-passwd, bundled com.rubix.tick-counter on a 5s cron)
- counter value is read off `[data-node-kind="starter.flow.counter"] .sf-slot__value` (BaseNode renders output slot value badges)
- the settings form field id `#setting-step` comes from `SettingsSidebar`'s `PrimitiveField` (`id={`setting-${name}`}`); the counter schema property is named `step`
- e2e was NOT executed against a running backend in this worktree (no agent on :8088); stage description requires `pnpm --filter @nube/rubix-frontend e2e green against a running backend` — operator should run that locally

## Open questions

- if the bundled tick-counter flow uses a different node id than `count`, the heading regex in the sidebar selection assertion may need adjustment
