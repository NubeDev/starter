# Mobile — thin slice

The first deliverable: render **one** dashboard
(`dashboard.disk-overview`, the page the rubix-agent already
seeds and serves) end-to-end on iOS and Android.

Same discipline as the backend [THIN-SLICE.md](../THIN-SLICE.md):
one demo path exercises every layer in the stack so the boundaries
prove themselves before we scale to more pages.

## Why this page

- It already exists, seeded by rubix-agent, and is the canonical
  smoke test for the web SDUI pipeline (see the curl in the
  session log around `POST /api/v1/ui/resolve`).
- Its renderer kinds are `page`, `row`, `col`, `kpi`, `chart`
  (`grid` is **not** used by this page) and it exercises `static`
  slot-value bindings via `value.ref`. Five renderer kinds + one
  binding kind: the minimum that exercises layout, data, and a
  chart simultaneously. Everything else (forms, tables, tabs) is
  follow-up work, not first-light work.

## Block sequence

Each block is one PR. Each PR lands its own commit set, tests,
and design-doc updates per [NEW-SESSION.md §3](../../../NEW-SESSION.md#3--workflow-for-this-session).

### Block 1 — `@nube/starter-theme-tokens`

- Extract palette / density / radius / type / motion / role from
  `starter-ui-kit/src/styles/globals.css` and
  `starter-ui-core/src/theme-editor/presets.ts` into one JS object,
  one file per concept (see [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-theme-tokens)).
- Refactor `starter-ui-kit` to generate `globals.css` from the
  same object at build time. Web visual output unchanged — proven
  by snapshot diff in CI.
- Promote this file's section to `docs/design/mobile/packages.md`
  on merge.

### Block 2 — `@nube/starter-ui-kit-native`

- Implement the 13 primitives listed in
  [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-ui-kit-native), one
  file per primitive, ≤200 lines each.
- Story-style harness in `packages/starter-ui-kit-native/example/`
  so a reviewer can render each primitive in isolation against
  light / dark / two named palettes.
- Visual parity smoke test: side-by-side screenshots of the
  primitives versus the web kit at three breakpoints.

### Block 3 — `@nube/starter-ui-sdui-native` (first five kinds)

- Implement `render-page.tsx`, `render-row.tsx`, `render-col.tsx`,
  `render-kpi.tsx`, `render-chart.tsx` (5 of 16 web-registered
  kinds; the other 11 land in follow-up blocks, with the
  10-renderer parity gap vs the IR `Kind` union called out
  separately in [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-ui-sdui-native)).
  The slice only needs these five. `render-grid.tsx` lands in
  the first follow-up block, not the slice — `disk-overview` does
  not use it.
- Each renderer ≤150 lines. No direct RN primitives — only
  `starter-ui-kit-native`.
- Register from a single barrel `src/index.ts`. Importing the
  package from the mobile app is what registers everything.
- **Blocked by:** the `starter-ui-sdui-react/headless` split — see
  [NEW-PACKAGES §Precondition](./NEW-PACKAGES.md#precondition--sdui-react-package-split).

### Pre-Block 4 — backend bearer-token endpoint (LANDED)

Landed. `POST /api/v1/auth/token` in `starter-auth-users` accepts
`{ email, password, tenant_id? }` and returns `{ token,
expires_at, token_type: "Bearer" }`. Bearer **acceptance** was
already in place via
[`crates/starter-auth-users/src/principal_layer.rs`](../../../../crates/starter-auth-users/src/principal_layer.rs);
this route adds the missing **issuance** counterpart so mobile,
native-desktop, and CLI clients can sign in without cookies.
Location decision (live in `starter-auth-users`, not just
rubix-agent) and full payload + error contract are recorded in
[`docs/design/auth/token-issuance.md`](../../design/auth/token-issuance.md).

**Promotion note:** Block 4 can now proceed against this route.
On promotion of this file to `docs/design/mobile/`, this section
collapses to a one-line pointer at `docs/design/auth/token-issuance.md`.

### Block 4 — `rubix/mobile` scaffold + login + provider stack

- `pnpm create expo-app rubix/mobile -t expo-template-blank-typescript`.
- Wire the workspace deps and metro config per
  [APP-SHELL.md](./APP-SHELL.md#metro).
- Implement the token `AuthStrategy` (see
  [APP-SHELL.md](./APP-SHELL.md#strategy)).
- Implement the login screen on top of `useAuth`.
- Pre-Block 4 has landed — the route is live at
  `POST /api/v1/auth/token` (see
  [`docs/design/auth/token-issuance.md`](../../design/auth/token-issuance.md)).

### Block 5 — `dashboards/[pageId].tsx` + the slice itself

- Implement the dashboards route (`<SduiPage pageRef={pageId} />`)
  and the redirect from `/` to `/dashboards/disk-overview` (or to
  `/connections/new` if there is no active connection).
- Manual smoke test on iOS simulator + Android emulator: login →
  redirect → page renders with live data.
- Add an e2e on **Maestro** (chosen over Detox: no native build
  step, YAML flows, Expo-friendly) that performs the same flow.
- The slice is **done** when the same page renders correctly on
  both platforms with the same data the web shows.

## Out of scope for the slice

- The other 10 IR kind renderers — covered by follow-up blocks.
- `starter-ui-dashboard-native` widgets — `disk-overview` is built
  from generic IR kinds, not the bespoke widgets. Those land when
  a page needs them.
- Push notifications, offline cache, biometric unlock — all
  separate ADRs once the slice proves the chassis.

## Exit gate

The slice merges when:

1. CI green on `rubix/mobile` lint + tests + Maestro e2e.
2. The import-lint rule from [APP-SHELL.md](./APP-SHELL.md#import-lint)
   is in place and clean.
3. A signed **TestFlight (iOS) + Internal-track (Android)** build
   is produced via EAS and a link is attached to the PR. Bare
   screenshots are not enough — the binary must demonstrably
   reach the store-review pipeline.
4. Each block's design doc lives under `docs/design/mobile/`
   (per [README.md](./README.md#promotion-path)).
5. This file is **deleted** — the slice has shipped; the
   present-tense description is now in `docs/design/mobile/`.
