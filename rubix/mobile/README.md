# `@nube/rubix-mobile` — Expo / React Native shell

> Scope: thin-slice mobile chassis for connecting to one or more
> rubix-agent servers and rendering server-driven dashboards. The
> design docs live under [`rubix/docs/scope/mobile/`](../docs/scope/mobile/README.md);
> the same files will be promoted to `rubix/docs/design/mobile/` once
> the slice is integrated end-to-end.

This package is the **mobile thin-slice**:

1. ~~Block 0 — design docs landed~~
2. ~~Pre-Block 4 — `POST /api/v1/auth/token` endpoint~~ (commit `33ed0ca`)
3. ~~Block 4 — app shell~~: Expo SDK 54 + expo-router 6, local
   SQLite connection ledger, per-connection bearer auth via
   `expo-secure-store`, two-step login (`credentials` → `/auth/token`
   → install bearer), provider stack, login + connections screens.
4. ~~Block 5 — `<SduiPage>` renderer + `/dashboards/[pageId]`~~:
   `SduiPageNative` (loading / error / dry-run / ok states) wired to
   the dashboards route; `@nube/starter-ui-sdui-native` side-effect
   registers the 16 RN renderers in `app/_layout.tsx`; boot redirect
   resumes the last-opened page per connection.

**Exit-gate work remaining** (the slice is not "done" until):

- Maestro e2e under `rubix/mobile/e2e/` covering login → redirect →
  dashboard render.
- Manual smoke transcript captured on iOS simulator + Android
  emulator.
- EAS TestFlight (iOS) + Internal-track (Android) build link
  attached to the PR.
- Promote `rubix/docs/scope/mobile/*` → `rubix/docs/design/mobile/`
  (per [README.md](../docs/scope/mobile/README.md#promotion-path)).
- Delete `rubix/docs/scope/mobile/THIN-SLICE.md` once exit gate is
  green.

See [`THIN-SLICE.md` §Exit gate](../docs/scope/mobile/THIN-SLICE.md#exit-gate)
for the canonical checklist.

## What's in this package

```
src/
├─ app/                     ← Expo Router (file-based)
│  ├─ _layout.tsx           ← provider stack + SSE polyfill
│  ├─ index.tsx             ← boot redirect
│  ├─ login.tsx             ← two-step credentials → bearer
│  ├─ connections/{index,new,[id]}.tsx
│  ├─ dashboards/{index,[pageId]}.tsx
│  └─ settings.tsx          ← logout + connection switcher
├─ auth/
│  ├─ install.ts            ← installBearer / clearBearer
│  └─ strategy.ts           ← issueTokenForConnection / loginWithCredentials
├─ connection/
│  ├─ active-id-store.ts    ← zustand atom (single writer)
│  ├─ client-strap.tsx      ← mounts RubixClientProvider when active
│  └─ provider.tsx          ← reads DB, builds per-connection clients
├─ i18n/
│  ├─ provider.tsx          ← react-intl + OS locale detection
│  ├─ en.json
│  └─ es.json
├─ lib/
│  └─ client.ts             ← StarterClient/RubixClient factories
├─ local-db/                ← SQLite schema + verbs
│  ├─ migrations/           ← 3 SQL files + index
│  ├─ connection/           ← list/get/create/update/delete/active/touch/set-active
│  ├─ state/                ← last-page, last-sync
│  └─ token/                ← contract + expo-secure-store + get/put/clear
├─ sdui/
│  ├─ page.tsx              ← SduiPageNative (loading/error/dry-run/ok)
│  ├─ provider.tsx          ← SduiProvider wired to per-connection transport
│  └─ transport.ts          ← RN fetch/SSE transport
├─ state/
│  └─ pending-route.ts      ← survives 401 mid-session
├─ theme/
│  └─ provider.tsx          ← OS-driven light/dark tokens
└─ providers.tsx            ← 8-provider composition root
```

## The two-step login

The rubix backend's `/auth/me` is cookie-only — it returns 401 for
bearer requests, by design (bearer surface is `/api/v1/tools/*`). So
mobile cannot reuse the upstream `tokenStrategy.login` helper, which
probes `/auth/me` after installing the header. Instead:

1. `issueTokenForConnection({ baseUrl, email, password, tenantId? })`
   POSTs to `<baseUrl>/api/v1/auth/token` via plain `fetch`. The route
   landed in commit `33ed0ca` and is documented in
   [`token-issuance.md`](../docs/design/auth/token-issuance.md).
2. `installBearer({ client, secureStore, connectionId, token })` writes
   `Authorization: Bearer …` onto the in-memory `StarterClient` and
   mirrors the token to `expo-secure-store` (key
   `rubix.token.<connectionId>`) so a cold start can rehydrate.

`loginWithCredentials({...})` is the convenience wrapper used by the
login screen.

## Multi-connection isolation

One `StarterClient` + one `RubixClient` per active connection,
rebuilt on every `ConnectionProvider.setActiveId(...)`. The active id
is published through a module-level zustand atom
(`connection/active-id-store.ts`) that `starterQueryKey` consumers
namespace queries by — so the React Query cache stays warm across
switches without leaking rows between servers.

A single writer (`ConnectionProvider.setActiveId`) is the only thing
that calls `_setActiveIdInternal`. If you add a second writer, the
isolation guarantee silently breaks; update this README and
[`APP-SHELL.md`](../docs/scope/mobile/APP-SHELL.md) at the same time.

## Running locally

This package is part of the pnpm workspace. From the repo root:

```bash
pnpm install
pnpm --filter @nube/rubix-mobile typecheck    # tsc -b
pnpm --filter @nube/rubix-mobile lint
pnpm --filter @nube/rubix-mobile start        # expo dev server
```

The Metro config watches the repo root and disables hierarchical
lookup so the workspace `@nube/*` packages resolve correctly.

## Configuration

Connections are stored in SQLite — there is no app-level "base URL"
constant. On first launch the operator is sent to
`/connections/new` to register a server. The probe POST is best-
effort; an unreachable server is still saved (mobile-data toggle,
VPN dropped).

## Out of scope for the thin-slice

- The other 10 IR kind renderers — `starter-ui-sdui-native` ships
  the 16 that match the web set; the remaining `stack, card, text,
  heading, badge, kpi_grid, button, link, field, sparkline` are
  deferred-with-web (see
  [`NEW-PACKAGES.md` §Parity vs the IR Kind union](../docs/scope/mobile/NEW-PACKAGES.md)).
- Tenant picker UI for the 409 `tenant_required` response — the
  login screen surfaces the message but does not yet enumerate
  memberships. Tracked in
  [`THIN-SLICE.md`](../docs/scope/mobile/THIN-SLICE.md).
- Bespoke `starter-ui-dashboard-native` widgets on a page —
  `disk-overview` is built from generic IR kinds; widgets land when
  a page needs them.
- Push notifications, biometric unlock, offline reads, code-push,
  TLS pinning. See [`NON-GOALS.md`](../docs/scope/mobile/NON-GOALS.md).
- Replacing the in-house kit with Tamagui or gluestack — evaluated
  and recommended against for now; see
  [`UI-FOUNDATION-OPTIONS.md`](../docs/scope/mobile/UI-FOUNDATION-OPTIONS.md).

## Reference docs

- [`THIN-SLICE.md`](../docs/scope/mobile/THIN-SLICE.md) — block plan
- [`APP-SHELL.md`](../docs/scope/mobile/APP-SHELL.md) — provider stack
- [`local-db.md`](../docs/design/mobile/local-db.md) — SQLite schema (promoted from scope)
- [`REUSE.md`](../docs/scope/mobile/REUSE.md) — upstream package map
- [`NEW-PACKAGES.md`](../docs/scope/mobile/NEW-PACKAGES.md) — RN-only
  packages introduced for the slice
- [`token-issuance.md`](../docs/design/auth/token-issuance.md) — design
  note for `POST /auth/token`
