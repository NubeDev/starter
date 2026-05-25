# Mobile — non-goals

Things the mobile app explicitly does **not** ship. Listed by
name so reviewers can reject scope creep with a one-line link.

## Surfaces

| Excluded surface | Where it lives on web | Reason |
|---|---|---|
| Flow editor | [`rubix/frontend/src/routes/flows/`](../../../frontend/src/routes/flows/) | Pan/zoom node graph; not a phone-shaped interaction. |
| Warehouse admin | [`rubix/frontend/src/components/admin/warehouse/`](../../../frontend/src/components/admin/warehouse/) | Data-heavy admin; operators use the web app for this. |
| Extensions admin | [`rubix/frontend/src/routes/extensions.tsx`](../../../frontend/src/routes/extensions.tsx) | Same as above. |
| Tenant / team / authz admin | [`@nube/starter-ui-authz`](../../../../packages/starter-ui-authz/) | Same as above. |
| AI builder | [`@nube/starter-ui-ai-builder`](../../../../packages/starter-ui-ai-builder/) | Split-pane editor; not a phone-shaped interaction. |
| Print / export to PDF | [`@nube/starter-ui-export`](../../../../packages/starter-ui-export/) | DOM-only stack (`html2canvas` + `jspdf`). If mobile needs export, that's a separate share-sheet feature with its own ADR. |

## Technical

- **No web bundle.** `rubix/mobile` ships native binaries via Expo
  EAS. We don't add Expo Web — that would muddy the seam with the
  existing Vite SPA.
- **No code-push / OTA before launch.** Expo Updates may come
  later, but the slice ships through the store review pipeline so
  we measure the real-world boot path.
- **No biometric unlock in v1.** Token in AsyncStorage is the
  baseline; biometrics is a follow-up ADR after the slice ships.
- **No offline cache in v1.** React-Query cache is in-memory.
  Persistence is a follow-up once we understand which pages
  operators actually use on flaky connections.
- **No push notifications in v1.** SSE on a foregrounded screen
  is the slice; APNs / FCM is a separate workstream.
- **No second SDUI renderer set.** `@nube/starter-sdui-react` (the
  D2 / older renderers) stays web-only. Mobile maintains one
  renderer surface, period.
- **No bespoke per-page React Native screens beyond the dashboard
  route.** Everything is SDUI-driven; if a screen needs a hand
  layout, that's a sign the IR needs a new kind, not a new screen.

## Process

- **No "phase 2" markers in code.** Per
  [NEW-SESSION.md §2](../../../NEW-SESSION.md#2--the-non-negotiables),
  comments describe the code as it is now. Excluded surfaces are
  recorded **here**, not as `// TODO: ship in mobile v2` strings
  scattered through the source tree.
- **No mobile-only forks of shared packages.** If mobile needs a
  prop the web doesn't have, it lands on the web component first
  (see [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-ui-dashboard-native)).
- **No bypassing the import-lint rule.** A `// eslint-disable`
  on the forbidden-imports rule is a PR blocker.
