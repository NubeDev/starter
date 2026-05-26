# Flutter app — thin slice

The first deliverable: a Flutter app that boots on iOS, Android,
and the browser; lets the user save a rubix-agent connection; logs
in; and lands on **one hardcoded** dashboard screen with live data
from that agent.

**No SDUI in v1.** The home screen is a hand-written Flutter
screen that calls a small fixed set of REST endpoints (pinned in
Block 5 below). The whole point of the slice is to prove the
chassis (drift on all three platforms, dio + retrofit, auth
across the `kIsWeb` boundary, navigation, theme, i18n) — not the
IR plumbing. SDUI lands in v2.

**Six blocks, not five.** The original draft had Block 5 ending
at "green integration test + signed store-pipeline build." Those
are separate concerns: code-correctness vs first-time-signing
ops. The signing work is now Block 6, leaving Block 5 to end at
green integration tests and successful debug builds on all three
platforms.

Same discipline as the React Native plan in
[`docs/scope/mobile/THIN-SLICE.md`](../../scope/mobile/THIN-SLICE.md):
one demo path exercises every layer in the stack so the boundaries
prove themselves before we scale.

## Block sequence

Each block is one PR. Each PR ships its own tests, its own design
updates in this folder, and its own green CI on all three
platforms.

### Block 1 — scaffold + drift + web wasm setup

- `flutter create rubix/flutter --org io.nube --platforms ios,android,web --template app`.
- Wire pinned dependencies from [PACKAGES.md](./PACKAGES.md).
- Set up `build.yaml` and run `build_runner` once to confirm
  codegen works end-to-end (one trivial `@freezed` class is
  enough at this stage).
- Define `connections` and `connection_state` tables in drift,
  generate `app_database.g.dart`, write a unit test that opens
  the DB and inserts a row.
- Download `sqlite3.wasm` and `drift_worker.dart.js` into `web/`.
  Add `lib/core/storage/_drift_assets_version.dart` exporting
  `driftAssetsReleaseTag` (the release the assets came from) and
  `driftAssetsCompatRange` (the inclusive `(min, max)` band of
  drift dependency versions known wasm-ABI-compatible with those
  assets). See
  [DECISIONS](./DECISIONS.md#sqlite-library-drift-not-sqflite)
  for the band-vs-strict-equality rationale.
- Add a CI step (`tools/check_drift_assets.sh` or equivalent)
  that parses the resolved `drift` version from `pubspec.lock`,
  asserts band-membership against `driftAssetsCompatRange`, and
  asserts both wasm/worker files exist. Catches drift bumps that
  forget to refresh `web/` *and* catches stale asset bands
  without flagging routine patch bumps.
- Verify the app boots on:
  - iOS simulator
  - Android emulator
  - Chrome (via `flutter run -d chrome`)
- **Acceptance:** an empty screen with the drift DB initialized
  and one provider exposing it, on all three platforms.

### Block 2 — connections CRUD

- `Connection` freezed model: `id`, `label`, `baseUrl`,
  `createdAt`, `lastUsedAt`. `lastUsedAt` is bumped by
  `ConnectionRepository.markUsed(id)`, called from the Block 3
  login success handler — not by any read path. v1 surfaces it
  only as a sort key in the connections list.
- `ConnectionDao` with insert / list / update / delete /
  setActive / markUsed.
- `ConnectionRepository` over the DAO; `connectionListController`
  and `activeConnectionProvider` exposing it.
- Screens: `/connections` (list), `/connections/new` (add),
  `/connections/:id` (edit, delete). Plain Material 3 widgets, no
  custom kit yet.
- `/connections/new` probes `<baseUrl>/healthz` before saving,
  surfacing failures inline. This is the first real network call
  and exercises the dio pipeline end-to-end.

  **Auth posture.** `/healthz` is mounted **outside** the auth
  sandwich on `rubix-agent` (confirmed in
  [`rubix/crates/rubix-agent/src/main.rs`](../../../crates/rubix-agent/src/main.rs)
  — the `healthz_router()` is merged before the DSN-gated
  auth+authz+changelog layers wrap the rest of the surface). The
  probe therefore runs on a `Dio` instance built **without** the
  `AuthInterceptor` (which doesn't exist until Block 3 anyway).
  Block 2 ships a small `probeDio()` factory in
  `core/network/dio_client.dart` that returns a bare `Dio` for
  this single use; Block 3's full `dioProvider` is built on top
  of it by adding interceptors.
- **Acceptance:** add a connection by URL, see it in the list,
  edit its label, delete it. State survives a restart on all
  three platforms (web stores in drift's OPFS or IndexedDB
  fallback per `drift_flutter`).

### Block 3 — auth (issue + install + 401)

- `LoginRequest` / `LoginResponse` freezed DTOs.
- `ApiClient.login(LoginRequest)` retrofit method against
  `POST /api/v1/auth/token`.
- `TokenStore` interface; `MobileTokenStore` (flutter_secure_storage)
  and `WebTokenStore` (in-memory). `kIsWeb` branch in
  `tokenStoreProvider`.
- `AuthInterceptor`: injects bearer, handles 401 by evicting and
  routing to `/login`.
- `/login` screen: email + password fields, "Sign in" button,
  inline error. Two-step exchange per
  [APP-SHELL §Auth](./APP-SHELL.md#auth).
- `go_router` redirect logic for splash → connections-new /
  login / home.
- **Acceptance:** add a real rubix-agent connection, log in,
  cold-start the app, land on `/home` on mobile (token survives)
  and on `/login` on web (token does not). 401 from any call
  routes back to login.

### Block 4 — theme + i18n + the polish pass

- Material 3 light/dark theme in `app_theme.dart`, seeded from the
  same primary as `rubix/frontend`.
- `theme_providers.dart` reading mode from `shared_preferences`
  (`system` / `light` / `dark`); `/settings` screen toggles it.
- `gen-l10n` with `app_en.arb` and `app_es.arb` covering the
  ~20 strings the v1 screens use. Locale picker in `/settings`.
- `LoadingIndicator`, `ErrorPanel`, `UnreachablePanel` shared
  widgets used by the connections and login screens.
- A formal pass on visual polish: spacing, typography scale,
  empty states, error copy. This is the "look and feel"
  milestone the user called out — v1 ships looking finished, not
  prototype.
- **Acceptance:** the app looks intentional in light and dark on
  all three platforms; English and Spanish both render; settings
  changes persist across restarts.

### Block 5 — home screen + integration test

- `HomeScreen`: one hand-written Flutter screen that calls
  **exactly these endpoints**, pinned now so the block has known
  shape:
  - `GET /healthz` — unauth, confirms the agent is reachable.
    Renders a green/red status pill.
  - `GET /api/v1/auth/me` — auth-required (forces a real
    interceptor path), returns the signed-in user identity per
    [`crates/starter-auth-users/src/routes/me.rs`](../../../../crates/starter-auth-users/src/routes/me.rs).
    Renders email + display name.
  - Active connection's `label` + `baseUrl` from the local DB —
    proves the multi-instance plumbing terminates on the screen.

  These three together exercise every layer the chassis defines:
  drift read, unauth network call, auth network call (with
  bearer injection and 401 handling), Riverpod fan-in, theme,
  i18n. **Note:** `rubix/frontend`'s logged-in landing is
  `/dashboards/disk-overview` which is an SDUI page — there is no
  non-SDUI dashboard for the Flutter app to mirror, which is
  exactly why v1 stops at chassis validation rather than
  attempting dashboard parity. See
  [README §Why "foundations first"](./README.md#why-foundations-first--v1-as-a-chassis-validation-milestone).

  Loading / error / unreachable states wired up using the shared
  widgets from Block 4.
- An `integration_test/smoke_test.dart` that:
  1. Launches the app.
  2. Adds a connection (env-supplied URL).
  3. Logs in with env-supplied creds.
  4. Asserts the home screen renders: status pill green, signed-in
     identity matches the env-supplied email.
- CI runs this on iOS simulator, Android emulator, and headless
  Chrome.
- **Acceptance — Block 5 done when:**
  1. The integration test is green on all three platforms in CI.
  2. The drift assets version-pin CI check is green.
  3. `flutter analyze` is clean (zero warnings).
  4. **Debug** builds are produced and `flutter run` succeeds on
     all three targets. (Signed release builds are Block 6.)

### Block 6 — store-pipeline signing + first internal release

Split out from Block 5 because first-time iOS signing is a
multi-day operations task that has nothing to do with the chassis
the slice is validating. Bundling it into Block 5 would conflate
"the code is right" with "the org has Apple Developer Program
membership wired up," and the second one can block the first for
reasons no PR review can fix.

- iOS: provisioning profile, distribution certificate, App Store
  Connect record, TestFlight metadata, first signed build pushed
  to internal testers.
- Android: upload key + Play app signing, Internal testing track
  created, first signed AAB uploaded.
- Web: a static-hosted deploy target picked and configured
  (Cloudflare Pages, Netlify, or whatever the org already runs).
  Web deploys are cheap; this is a sanity check, not a release
  pipeline.
- A `BUILD.md` (or `docs/design/flutter/build.md`, promoted on
  merge) capturing the exact commands and the key/cert custodian
  for each platform — first-time signing pain is reducible to
  zero by writing it down.
- **Acceptance — v1 done when:**
  1. Block 5 acceptance still green.
  2. Signed iOS build live on TestFlight; signed Android build
     live on Play Internal; web build live at the chosen URL.
     Links attached to the PR.
  3. `BUILD.md` walks an org outsider through reproducing each
     build.
  4. This file collapses to a one-line pointer: "v1 shipped in
     PRs #N–#N+5; signing notes in build.md."

## Out of scope for the slice

- SDUI. The whole IR + renderer story is v2 (see
  [NON-GOALS.md](./NON-GOALS.md#out-of-scope-for-v1-but-explicit-v2-candidates)).
- Multiple dashboards. The home screen is one screen, pinned to
  three endpoints.
- Refresh tokens, biometric unlock, push notifications, offline
  cache, deep links to specific pages. All deferred.
- A bespoke component kit. v1 uses Material 3 widgets directly.
- OpenAPI-generated DTOs. v1 hand-mirrors the small set of DTOs
  it needs; the v2 SDUI work is what triggers codegen adoption.
- A release pipeline beyond first-internal-build. CI for `main`
  cutting a tagged release on every merge is a v2+ concern; Block
  6 establishes the manual signing path, no more.

## What "v1 done" buys us

A working three-platform Flutter app with:

- A real multi-connection model (drift + repositories).
- Real auth against a real backend, with platform-correct token
  handling.
- A polished look that we can show internally without disclaimers.

That foundation is the launchpad for v2 (SDUI), v3 (bespoke
dashboard widgets), and beyond. Each future block adds capability
without renegotiating the chassis.
