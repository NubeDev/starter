## Done

- Wired `<Empty>`/`<Skeleton>` from `@nube/starter-ui-kit` into the two remaining list-rendering rubix routes that still had plain-text placeholders: `rubix/frontend/src/routes/extensions.tsx` and `rubix/frontend/src/routes/admin/users.tsx`. Each now renders 3× `Skeleton` rows during `isLoading` and an `Empty` block (icon + title + description) when the list is empty.
- Added matching i18n keys `extensions.empty.title|body` and `users.empty.title|body` to both `rubix/frontend/src/i18n/en.json` and `es.json`.
- `pnpm --filter @nube/rubix-frontend typecheck` is green.
- Committed as `38d3bdb` with message starting `phase D.2 — toast-error-listener + empty states + loading skeletons — …`.

## Next

- (none — next stage is phase D.3, a fresh session will pick it up)

## What you need to know

- The other lists called out by the stage spec (flows index, $flowId, warehouse rules/marts/retention/insights) already used `Empty` + `Skeleton` from earlier stages — verified by grep; no edits needed there. The `/admin/access` route only mounts the black-box `<AuthzAdmin>` from `@nube/starter-ui-authz`, so its internal loading/empty UX lives in that package.
- Toast `onError` listener was **NOT** added. Per the stage's own SCOPE OQ-6 escape clause ("if not, raise BLOCKED — toasts shouldn't be hand-rolled in rubix"), I verified `packages/starter-ui-kit/src/components/ui/` contains no `toast.tsx` / `sonner.tsx`, and `src/index.ts` re-exports nothing toast-related (only `theme-editor-page.tsx` mentions "toast" in a comment, referring to a host-supplied callback). Recorded the BLOCKED reason in the commit body.
- All edits are pure consumption of existing `@nube/starter-ui-kit` exports — no starter package was modified or added.

## Open questions

- Unblocking the toast listener requires a separate `@nube/starter-ui-kit` job to add a Toast (likely Sonner-based) primitive. Once that lands, a follow-up rubix stage should create `rubix/frontend/src/components/toast-error-listener.tsx`, register a `QueryCache({ onError })` on the `QueryClient` in `main.tsx`, and filter for uncaught `RubixError` (importable from `@nube/rubix-client-react` or `@nube/rubix-client-ts`).
