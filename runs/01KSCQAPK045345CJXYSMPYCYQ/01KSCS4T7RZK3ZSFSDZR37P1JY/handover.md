## Done

- audited Phase B commits and ran tests for both client packages

## Next

- (none — gate decision below)

## What you need to know

- Five Phase B commits on master: 562ed87 (B.1 openapi refresh), 7216600 (B.2 rubix-client-ts extensions endpoint), 7a5191e (B.3 rubix-client-react scaffold), 74b3374 (B.4 read hooks), fa90227 (B.5 remaining hooks).
- Test counts (pnpm vitest --run): `@nube/rubix-client-ts` = 33 passed / 10 files; `@nube/rubix-client-react` = 51 passed / 12 files; combined = 84 passed.
- Extensions SSE wrapper against merged backend — CANNOT BE CONFIRMED. `rubix/openapi.json` contains zero "extension" references (`grep -c extension rubix/openapi.json` → 0). The rubix-extensions-wire PR is NOT on master, so the `/api/v1/extensions/events` route does not exist for any curl evidence. No `curl -N http://localhost:.../api/v1/extensions/events` line can be pasted.
- B.1 (stage 4) was specced to `raise BLOCKED with a one-paragraph status report and halt` if rubix-extensions-wire wasn't merged. It silently committed a handover.md only — no openapi refresh, no BLOCKED escalation.
- B.2 (stage 5) was specced to add `packages/rubix-client-ts/src/endpoints/extensions.ts` + `streamExtensionEvents`. The file does not exist (`find rubix/packages/rubix-client-ts/src -name 'extensions*'` empty). Stage committed only handover.md. The barrel `src/endpoints/index.ts` does not export extensions.
- B.4 worked around the missing typed methods: `rubix/packages/rubix-client-react/src/hooks/extensions.ts` calls `fetchJson` / `readCsrfHeader` directly against `/api/v1/extensions/*`, and `use-extension-events.ts` hard-codes the path. The hook header comment explicitly says "Until that ships these hooks talk to rubix-agent directly". This violates R1 (rubix-client-react is supposed to depend on rubix-client-ts typed methods, not bypass them) and R2/R5 (path strings duplicated in two layers; trust boundary leaks into the React layer).

## Open questions

- (none — sentinel below is the gate verdict)

FAIL: Phase B.1's precondition (extensions backend merged) was never satisfied, B.2's rubix-client-ts extensions endpoint + streamExtensionEvents file was never created, B.4 hooks bypass rubix-client-ts and call rubix-agent paths directly — wire-format duplication breaks R1/R2 and no curl evidence against the (still-unmerged) backend route is possible.
