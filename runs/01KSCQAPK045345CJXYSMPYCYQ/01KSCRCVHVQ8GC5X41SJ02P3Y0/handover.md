## Done

- Added `src/hooks/{system,users,mcp,extensions,use-extension-events}.ts` to `rubix/packages/rubix-client-react` with sibling `.test.tsx` files; barrel re-exports them via `src/index.ts`.
- Shared `src/hooks/test-harness.tsx` mounts `QueryClientProvider` + `RubixClientProvider` over a fetch-stub `StarterClient`; retries disabled so errors surface in tests.
- `pnpm --filter @nube/rubix-client-react typecheck` and `test` green (31 tests across 6 files).
- Committed as `stage 8: phase B.4 — rubix-client-react read hooks (system + users + tools/list + extensions)`.

## Next

- (none — next session picks up stage 9)

## What you need to know

- Stage 5 (`phase B.2 — rubix-client-ts extensions endpoint + streamExtensionEvents`) appears to have landed only a handover commit; the typed `extensionsList`/`extensionsStart`/… methods and `streamExtensionEvents` do not exist on `RubixClient`. To unblock this stage the extension hooks talk to `/api/v1/extensions` and `/api/v1/extensions/{id}/{action}` directly through `fetchJson` + `readCsrfHeader` on `client.starter`, and `useExtensionEvents` calls `useEventStream` with the hard-coded path `/api/v1/extensions/events`. Shapes match the planned typed API so swapping each call to the typed method is a one-liner per hook once stage 5 actually ships.
- CSRF header is `X-CSRF-Token` (case-sensitive) — tests assert on that exact casing.
- Test harness stubs the CSRF cookie via `document.cookie = "starter_csrf=…"` (jsdom-friendly); replacing `globalThis.document` wholesale breaks `@testing-library/react`.

## Open questions

- Should a follow-up stage redo the missing stage 5 work (real typed extensions endpoint module + `streamExtensionEvents` in `rubix-client-ts`) and then migrate `extensions.ts`/`use-extension-events.ts` here onto the typed methods? Flagging for the operator.
