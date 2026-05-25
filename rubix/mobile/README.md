# `@nube/rubix-mobile` — Expo / React Native shell

> Scope: thin-slice mobile chassis for connecting to one or more
> rubix-agent servers and rendering server-driven dashboards. The
> design docs live under [`rubix/docs/scope/mobile/`](../docs/scope/mobile/README.md);
> the same files will be promoted to `rubix/docs/design/mobile/` once
> the slice is integrated end-to-end.

This package is **Block 4** of the mobile thin-slice:

1. ~~Block 0 — design docs landed~~
2. ~~Pre-Block 4 — `POST /api/v1/auth/token` endpoint~~ (commit `33ed0ca`)
3. **Block 4 — app shell** (this PR): Expo SDK 56 + expo-router 6,
   local SQLite connection ledger, per-connection bearer auth via
   `expo-secure-store`, two-step login (`credentials` → `/auth/token`
   → install bearer), provider stack, login + connections screens.
4. Block 5 — `<SduiPage>` renderer + the `/dashboards/[pageId]` body.

## What ships in this PR

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
├─ local-db/                ← SQLite schema + verbs (already landed)
│  ├─ migrations/           ← 3 SQL files + index
│  ├─ connection/           ← list/get/create/update/delete/active/touch/set-active
│  ├─ state/                ← last-page, last-sync
│  └─ token/                ← contract + expo-secure-store + get/put/clear
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

## Out of scope for Block 4

- The dashboard renderer (`<SduiPage>`). `/dashboards/[pageId]` is a
  stub that records `last_opened_page_ref` so deep-linking + resume can
  be verified end-to-end ahead of Block 5.
- Tenant picker UI for the 409 `tenant_required` response — the login
  screen surfaces the message but does not yet enumerate memberships.
  Tracked in [`THIN-SLICE.md`](../docs/scope/mobile/THIN-SLICE.md).
- Push notifications, biometric unlock, offline reads. See
  [`NON-GOALS.md`](../docs/scope/mobile/NON-GOALS.md).

## Reference docs

- [`THIN-SLICE.md`](../docs/scope/mobile/THIN-SLICE.md) — block plan
- [`APP-SHELL.md`](../docs/scope/mobile/APP-SHELL.md) — provider stack
- [`LOCAL-DB.md`](../docs/scope/mobile/LOCAL-DB.md) — SQLite schema
- [`REUSE.md`](../docs/scope/mobile/REUSE.md) — upstream package map
- [`NEW-PACKAGES.md`](../docs/scope/mobile/NEW-PACKAGES.md) — RN-only
  packages introduced for the slice
- [`token-issuance.md`](../docs/design/auth/token-issuance.md) — design
  note for `POST /auth/token`
