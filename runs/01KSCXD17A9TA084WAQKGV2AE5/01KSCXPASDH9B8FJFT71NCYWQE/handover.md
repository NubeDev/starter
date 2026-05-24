## Done

- Filled out the full `access.*` i18n keyspace in `rubix/frontend/src/i18n/en.json` and `es.json` covering every section of `AuthzMessages` (common, tenants, members, teams, rules, assignments, resources, check, decisions, including all nested columns/form/filters/teamMembers).
- Updated `rubix/frontend/src/routes/admin/access.tsx` so `useAuthzMessagesFromIntl()` now passes every section (not just `shell`) into `<AuthzAdmin i18n={...} />`. Previously only `shell.title` + `shell.tabs` were wired; the remaining sections silently fell back to `DEFAULT_AUTHZ_MESSAGES` English defaults, which is the (c)-class gap this stage is allowed to fix.
- Committed as `phase A.1 — authz tab triage …` on branch `codeless/rubix-frontend-surfaces` (commit `babb4c0`).

## Next

- Stage 2 should pick up the next phase A item per SCOPE.md. A fresh session will resume.

## What you need to know

- Runtime walk-through of the 8 tabs against `make start` was **not** performed — this worktree is headless, no browser/backend was booted. The triage was performed statically against `packages/starter-ui-authz/src/panels/*`, `packages/starter-client-ts`, and `rubix` agent endpoints from the file tree.
- The mergeAuthzMessages() helper in `packages/starter-ui-authz/src/i18n/messages.ts` deep-merges over defaults, so the new wiring is safe even if a key is missing — the route now provides every key explicitly though.
- JSON validity for both locale files was verified with `python3 -c "json.load(...)"`.

## Open questions

- (a)-class follow-up: confirm that rubix-agent actually exposes the eight authz endpoints `AuthzAdmin` calls (tenants, teams, members, rules, assignments, resources, check, decisions). Static audit not done here — should be the first thing a runtime walk-through verifies in a non-headless session.
- (b)-class follow-up: confirm `StarterClient` (packages/starter-client-ts) implements every method the eight panels invoke (`tenants-panel.tsx` … `decisions-panel.tsx`). Static cross-check was deferred; a quick grep of `client\.` calls inside each panel vs. the StarterClient method list will surface any missing method.
- Localisation of `roleLabels` / `resourceLabels` (optional overrides) was intentionally skipped — those are content-driven and not part of the static AuthzMessages keyspace.
