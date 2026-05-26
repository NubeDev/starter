# Flutter app — implementation stages

A linear, tickable checklist for v1. One section per block from
[THIN-SLICE.md](./THIN-SLICE.md), broken into atomic items an
agent (or human) can mark `[x]` as each one lands.

## How to use this file

- **Tick exactly when the item is true in `master`.** Not on PR
  open, not on local commit. `[x]` means "merged and CI green."
- **One item per commit-able unit of work.** If an item is too big
  to fit in one PR, split it before starting.
- **Do not skip ahead.** Items within a block can sometimes be
  reordered, but blocks themselves are sequential. Block N+1
  assumes Block N's invariants hold.
- **Acceptance gates are not items.** They are conditions to
  evaluate after the items in a block are all ticked. A block is
  not "done" until both the item list and the acceptance gate
  pass. The gate gets its own checkbox at the end of each block.
- **Verify before ticking.** "Verify against current code" is the
  rule. Running `flutter analyze`, `flutter test`, and the
  block's named CI checks is the minimum bar before any tick.
- **When unsure, leave it pending.** A wrongly-ticked item makes
  the next block start on a false floor.

## Status legend

- `[ ]` — pending, not started or in progress
- `[x]` — done and merged to `master`
- `[~]` — in progress (used sparingly; prefer `[ ]` unless the
  item has been actively touched in the last 48h)
- `[!]` — blocked; must include a one-line note pointing at the
  blocker (`[!] blocked on <issue/PR/decision>`)

---

## Block 0 — Pre-flight (not in THIN-SLICE; landed before Block 1)

The scope itself and any backend prerequisites.

- [x] Scope drafted under `rubix/docs/design/flutter/`
  (README, DECISIONS, APP-SHELL, PACKAGES, THIN-SLICE,
  NON-GOALS, this file).
- [x] Backend `POST /auth/token` route live in
  `crates/starter-auth-users/src/routes/token.rs`.
- [x] Backend bearer acceptance live in
  `crates/starter-auth-users/src/principal_layer.rs`.
- [x] Backend `GET /auth/me` route live in
  `crates/starter-auth-users/src/routes/me.rs`.
- [x] Backend `GET /healthz` mounted outside auth sandwich
  (verified in `rubix/crates/rubix-agent/src/main.rs`).
- [ ] Scope reviewed and approved by at least one reviewer outside
  the drafter.

---

## Block 1 — Scaffold + drift + web wasm setup

**Goal:** an empty Flutter app that boots on all three platforms
with the drift database initialized.

### Repo + tooling

- [ ] `rubix/flutter/` directory created via
  `flutter create rubix/flutter --org io.nube --platforms ios,android,web --template app`.
- [ ] `pubspec.yaml` populated with dependencies from
  [PACKAGES.md](./PACKAGES.md). Versions match exactly.
- [ ] `analysis_options.yaml` enables `very_good_analysis` +
  `riverpod_lint`. Per-rule disables permitted per
  [DECISIONS §Lint set](./DECISIONS.md#lint-set-very_good_analysis-with-a-per-rule-disable-budget);
  any disable carries a one-line comment explaining why.
  Swapping lint sets is **not** allowed at Block 1.
- [ ] [FILE-LAYOUT.md](./FILE-LAYOUT.md) read by the Block 1
  implementer before the first source file lands. The 400-line
  ceiling, the verb-per-file pattern, and the
  `utils`/`helpers`/`widgets` taboo are enforced from commit one.
- [ ] `tools/check_file_size.dart` (or shell equivalent) in CI
  fails on any `lib/**/*.dart` or `test/**/*.dart` over 400 lines
  excluding generated suffixes. See
  [FILE-LAYOUT §9](./FILE-LAYOUT.md#9-enforcement).
- [ ] `build.yaml` configured for one-pass codegen
  (freezed, json_serializable, retrofit_generator, drift_dev,
  riverpod_generator).
- [ ] `dart run build_runner build --delete-conflicting-outputs`
  succeeds on a fresh checkout.
- [ ] One trivial `@freezed` class in `lib/` proves codegen runs
  end-to-end.

### Drift schema + DB

- [ ] `lib/core/storage/tables/connections_table.dart` defines
  the `connections` table.
- [ ] `lib/core/storage/tables/connection_state_table.dart`
  defines the single-row `connection_state` table.
- [ ] `lib/core/storage/app_database.dart` defines
  `@DriftDatabase` over those tables, `schemaVersion = 1`.
- [ ] `app_database.g.dart` regenerates without warnings.
- [ ] Unit test opens the DB, inserts a `connections` row,
  reads it back. Test passes on native (mobile/desktop test
  runner).

### Web wasm assets + version-pin canary

- [ ] `web/sqlite3.wasm` downloaded from the drift release
  matching the resolved `drift` version. File committed to git.
- [ ] `web/drift_worker.dart.js` from the same release. Committed.
- [ ] `lib/core/storage/_drift_assets_version.dart` exports
  `driftAssetsReleaseTag` (the release the assets came from) and
  `driftAssetsCompatRange` (the inclusive `(min, max)` band of
  drift dependency versions known wasm-ABI-compatible with those
  assets). See
  [DECISIONS](./DECISIONS.md#sqlite-library-drift-not-sqflite).
- [ ] App boot in debug asserts the resolved `drift` version
  falls inside `driftAssetsCompatRange`.
- [ ] `tools/check_drift_assets.sh` (or equivalent) parses the
  resolved `drift` version from `pubspec.lock`, checks
  band-membership against `driftAssetsCompatRange`, exits
  non-zero on miss. Also asserts both `web/sqlite3.wasm` and
  `web/drift_worker.dart.js` exist.
- [ ] CI runs the script on every PR.

### Provider stack stub

- [ ] `lib/main.dart`: `runApp(ProviderScope(child: RubixApp()))`.
- [ ] `lib/app.dart`: `MaterialApp` (not `MaterialApp.router` yet
  — router comes in Block 3) with one empty screen.
- [ ] `appDatabaseProvider` (`keepAlive`) opens the drift DB and
  is consumed by the empty screen, which renders a count of
  rows in `connections` (0 on first launch).

### Boot verification (manual, named)

- [ ] iOS simulator: `flutter run -d <ios-sim>` succeeds, screen
  shows "0 connections."
- [ ] Android emulator: same.
- [ ] Chrome: `flutter run -d chrome` succeeds, screen shows
  "0 connections." (Web wasm path exercised.)

### Block 1 acceptance gate

- [ ] All items above ticked.
- [ ] `flutter analyze` clean (zero warnings).
- [ ] `flutter test` green.
- [ ] Drift assets CI check green.
- [ ] **Block 1 done.**

---

## Block 2 — Connections CRUD

**Goal:** the user can add, list, edit, delete, and activate
rubix-agent connections. State survives restart on all three
platforms.

### Models + repository

- [ ] `lib/features/connections/domain/connection.dart`: freezed
  `Connection` (`id`, `label`, `baseUrl`, `createdAt`,
  `lastUsedAt`).
- [ ] `lib/core/storage/daos/connection_dao.dart`: insert / list /
  update / delete / setActive / markUsed methods.
- [ ] `lib/features/connections/data/connection_repository.dart`:
  thin wrapper exposing the DAO via the domain model. `markUsed`
  is callable; nothing in Block 2 calls it (Block 3 will).
- [ ] `connectionListControllerProvider` (`@riverpod`) exposes
  the connections list as an `AsyncValue<List<Connection>>`.
- [ ] `activeConnectionProvider` (`@riverpod`) reads the active
  id from `connection_state` and resolves to `Connection?`.

### Probe network call

- [ ] `lib/core/network/dio_client.dart` exposes a `probeDio()`
  factory that returns a bare `Dio` with no interceptors (per
  THIN-SLICE Block 2 auth-posture note).
- [ ] `connection_repository.probe(baseUrl)` calls
  `GET <baseUrl>/healthz` via `probeDio()` and returns a
  typed result (ok / timeout / non-2xx / network error).

### Screens

- [ ] `lib/features/connections/presentation/connections_list_screen.dart`:
  list with tap-to-activate, swipe-to-delete, FAB to add.
- [ ] `lib/features/connections/presentation/add_connection_screen.dart`:
  URL + label form, probes before saving, surfaces probe failures
  inline.
- [ ] `lib/features/connections/presentation/edit_connection_screen.dart`:
  edit label, delete with confirm.
- [ ] Plain Material 3 widgets only. No custom theme yet (Block 4).

### Cold-start behavior

- [ ] App boots into `connections_list_screen` if no active
  connection, or a stub home if there is one. (Router lands in
  Block 3; for Block 2, a simple top-level `if/else` in
  `app.dart` is sufficient.)
- [ ] After adding a connection, restarting the app shows it in
  the list on all three platforms (drift OPFS/IndexedDB on web).

### Block 2 acceptance gate

- [ ] All items above ticked.
- [ ] `flutter analyze` clean.
- [ ] `flutter test` green (includes new repository tests with
  `mocktail` over a mocked DAO + a fake `Dio`).
- [ ] Manual smoke: add → list → edit → delete → activate, on
  iOS, Android, Chrome. State persists across restart on all
  three.
- [ ] **Block 2 done.**

---

## Block 3 — Auth (issue + install + 401)

**Goal:** the user can log in against the active connection. The
bearer is installed on the dio pipeline. 401 routes back to login
without losing the user's intended destination.

### DTOs + API client

- [ ] `lib/features/auth/data/dto/login_request.dart` freezed:
  `{ email, password, tenantId? }`.
- [ ] `lib/features/auth/data/dto/login_response.dart` freezed:
  `{ token, expiresAt, tokenType }`.
- [ ] `lib/core/network/api_client.dart`: `@RestApi` abstract
  class with `login(LoginRequest)` against `POST /api/v1/auth/token`
  and `me()` against `GET /api/v1/auth/me`. Codegen produces
  `api_client.g.dart` cleanly.

### Token store

- [ ] `lib/core/auth/token_store.dart` defines the interface.
- [ ] `lib/core/auth/token_store_mobile.dart` implements over
  `flutter_secure_storage`.
- [ ] `lib/core/auth/token_store_web.dart` implements as an
  in-memory holder.
- [ ] `lib/core/auth/token_store_providers.dart` branches on
  `kIsWeb`, exposes `tokenStoreProvider`.

### Dio interceptor stack

- [ ] `lib/core/network/auth_interceptor.dart`:
  - Reads the current token at request time via a captured
    `Ref` (not a constructor-time snapshot) per
    [APP-SHELL §Why the dioProvider does not rebuild on token change](./APP-SHELL.md#why-the-dioprovider-does-not-rebuild-on-token-change).
  - Injects `Authorization: Bearer <token>` if present.
  - On 401 from a non-exempt path: evicts the token under a
    module-level `_evictionLock` so stampedes evict exactly once.
  - Maintains a `Set<String>` of auth-exempt paths
    (`/api/v1/auth/token`, `/api/v1/auth/logout`, `/healthz`);
    401s from these pass through as normal errors. See
    [APP-SHELL §401 re-entrancy](./APP-SHELL.md#401-re-entrancy).
- [ ] `lib/core/network/log_interceptor.dart`: `pretty_dio_logger`,
  registered only when `kDebugMode`.
- [ ] `dioProvider` constructs `Dio` from active connection's
  `baseUrl` **only**. Rebuilt by Riverpod only when `baseUrl`
  changes. Token changes do **not** invalidate the instance —
  see the APP-SHELL link above for why.
- [ ] `apiClientProvider` provides `ApiClient(dio)`.
- [ ] `authRepositoryProvider` wraps `ApiClient` with the
  two-step login (issue → install) per
  [APP-SHELL §Auth](./APP-SHELL.md#auth). On success, calls
  `connectionRepository.markUsed(id)`.
- [ ] `authRepository.logout()` clears `pendingRouteProvider`
  before evicting the token (per the explicit-logout-vs-eviction
  semantics in [APP-SHELL §401 re-entrancy](./APP-SHELL.md#401-re-entrancy)).

### Router

- [ ] `lib/core/router/app_router.dart` configures `go_router`
  with the routes listed in
  [APP-SHELL §Navigation](./APP-SHELL.md#navigation).
- [ ] Splash redirect logic per APP-SHELL: no active connection
  → `/connections/new`; active but no token → `/login`; both
  → `/home` (stub for now; Block 5 fills it).
- [ ] `refreshListenable` wired to a `Listenable` that fans in
  `activeConnectionProvider` + `tokenStoreProvider` changes.
  (Implementer picks the Riverpod-3 idiom per
  [DECISIONS](./DECISIONS.md).)
- [ ] `pendingRouteProvider` stores the location an interrupted
  navigation was heading for; restored after re-login. 404 from
  the restored route → fall back to `/home` and toast.

### Login screen

- [ ] `lib/features/auth/presentation/login_screen.dart`: email
  + password fields, sign-in button, inline error, loading state.
- [ ] `lib/features/auth/presentation/login_controller.dart`
  (`@riverpod`) drives the two-step exchange.

### Cold-start behavior per platform

- [ ] Mobile: after a successful login, killing and reopening
  the app lands on `/home` (token survived in Keychain/Keystore).
- [ ] Web: after a successful login, refreshing the tab lands
  on `/login` (token did not survive — by design).

### Block 3 acceptance gate

- [ ] All items above ticked.
- [ ] `flutter analyze` clean.
- [ ] `flutter test` green, with explicit interceptor tests for:
  - Bearer injection when token present; no header when absent.
  - 401 stampede: 10 concurrent requests all 401, eviction
    fires exactly once.
  - Recursive 401: `/api/v1/auth/token` returns 401, error
    propagates to the login screen as a credentials failure, no
    redirect loop.
  - Logout vs eviction: `AuthRepository.logout()` clears
    `pendingRouteProvider`; interceptor-driven eviction
    preserves it.
- [ ] Manual smoke: log in against a real `rubix-agent`, cold
  start, observe the per-platform cold-start behavior above.
- [ ] Manual 401 test: revoke the token server-side (or wait
  past `expires_at`), navigate to any auth-gated screen,
  verify router lands on `/login` and the pending route is
  restored after re-login.
- [ ] **Block 3 done.**

---

## Block 4 — Theme + i18n + the polish pass

**Goal:** the app looks intentional in light and dark, on all
three platforms, in English and Spanish. Settings changes
persist.

### Theme

- [ ] `lib/core/theme/app_theme.dart`: light and dark
  `ThemeData` from `ColorScheme.fromSeed(seedColor: Color(0xFF1F2A2E))`
  per [DECISIONS §Theme seed color](./DECISIONS.md#theme-seed-color).
- [ ] `themeProvider` reads mode from `shared_preferences`
  (`system` / `light` / `dark`).
- [ ] `app.dart` switched to `MaterialApp.router` (was plain
  `MaterialApp` after Block 3) reading `themeMode` from
  `themeProvider`.
- [ ] `/settings` screen renders a segmented control for theme
  mode and persists changes.

### i18n

- [ ] `flutter gen-l10n` configured in `pubspec.yaml`.
- [ ] `lib/core/i18n/arb/app_en.arb` covers every visible string
  in the v1 screens (connections list/add/edit, login, settings,
  home stub, shared error/empty/loading widgets).
- [ ] `lib/core/i18n/arb/app_es.arb` same key set, Spanish
  translations.
- [ ] `localeProvider` reads from `shared_preferences`.
- [ ] All screens use `AppLocalizations.of(context)`; no string
  literals in widget trees.
- [ ] `/settings` exposes the locale picker.

### Shared widgets

- [ ] `lib/shared/widgets/loading_indicator.dart`.
- [ ] `lib/shared/widgets/error_panel.dart`.
- [ ] `lib/shared/widgets/unreachable_panel.dart` (Block 5 home
  uses it; Block 4 lands the widget itself).
- [ ] Connection and login screens replaced their hand-rolled
  loading/error UI with these widgets.

### Polish pass

- [ ] Visual review of every screen in light and dark, on iOS,
  Android, Chrome. Reviewer signs off in PR.
- [ ] Spacing, typography scale, empty states, error copy all
  reviewed and adjusted.
- [ ] App icon present per [APP-SHELL §Assets](./APP-SHELL.md#assets);
  `flutter_launcher_icons` config wired and outputs committed.
- [ ] Splash screen present; `flutter_native_splash` config
  wired and outputs committed.

### Block 4 acceptance gate

- [ ] All items above ticked.
- [ ] `flutter analyze` clean.
- [ ] `flutter test` green.
- [ ] The app looks intentional (not prototype) on all three
  platforms in both themes and both locales.
- [ ] **Block 4 done.**

---

## Block 5 — Home screen + integration test

**Goal:** chassis validation. The pinned three-call home screen
renders against a real agent on all three platforms. Integration
test green.

### Home screen

- [ ] `lib/features/home/presentation/home_controller.dart`
  (`@riverpod`) exposes:
  - `agentHealthProvider` calling `GET /healthz` via
    `probeDio()` (no bearer required).
  - `currentUserProvider` calling `GET /api/v1/auth/me` via
    `apiClientProvider`.
  - The active connection (from `activeConnectionProvider`).
- [ ] `lib/features/home/presentation/home_screen.dart` renders:
  - Status pill for agent health (green ok, red unreachable).
  - Email + display name from `/auth/me`.
  - Active connection label + baseUrl.
  - Loading / error / unreachable states using the Block 4
    shared widgets.

### Integration test

- [ ] `integration_test/smoke_test.dart` covers:
  1. Launch app with env-supplied URL + creds.
  2. Add a connection.
  3. Log in.
  4. Assert home screen renders: status pill green; email
     matches env-supplied value.
- [ ] Env vars documented (which `--dart-define` flags the test
  needs).

### CI

- [ ] CI job runs integration test on iOS simulator.
- [ ] CI job runs integration test on Android emulator.
- [ ] CI job runs integration test on headless Chrome.
- [ ] All three jobs green on `master`.

### Debug builds

- [ ] `flutter build ios --debug` succeeds.
- [ ] `flutter build apk --debug` succeeds.
- [ ] `flutter build web` succeeds.

### Block 5 acceptance gate

- [ ] All items above ticked.
- [ ] Drift assets version-pin CI check green.
- [ ] `flutter analyze` clean.
- [ ] All three integration test CI jobs green.
- [ ] Debug builds succeed on all three targets.
- [ ] **Block 5 done.**

---

## Block 6 — Store-pipeline signing + first internal release

**Goal:** signed builds reach the store-review pipelines and a
public web URL. First-time signing pain captured in writing.

### iOS

- [ ] Apple Developer Program membership confirmed for the org.
- [ ] App ID / bundle ID registered in Apple Developer portal.
- [ ] Distribution certificate created and stored in the org's
  key custody.
- [ ] Provisioning profile generated and added to the project.
- [ ] App Store Connect record created with placeholder
  metadata (name, description, icon, screenshots).
- [ ] TestFlight internal testing group created.
- [ ] `flutter build ipa --release` succeeds with signing.
- [ ] First signed IPA uploaded to TestFlight.
- [ ] At least one internal tester can install via TestFlight
  and reach the home screen.

### Android

- [ ] Upload key generated and stored in key custody.
- [ ] Play Console app record created.
- [ ] Play App Signing enabled.
- [ ] Internal testing track created with at least one tester.
- [ ] `flutter build appbundle --release` succeeds with signing.
- [ ] First signed AAB uploaded to Internal Testing.
- [ ] At least one internal tester can install via Play and
  reach the home screen.

### Web

- [ ] Static-hosting target selected (Cloudflare Pages, Netlify,
  S3+CloudFront, or whatever the org runs).
- [ ] `flutter build web --release` succeeds.
- [ ] First deploy live at the chosen URL.
- [ ] Web sqlite3.wasm + drift_worker.dart.js served with correct
  MIME types (`application/wasm` + `application/javascript`).
- [ ] Loading the URL in a fresh browser reaches the login screen.

### Documentation

- [ ] `docs/design/flutter/build.md` walks an outsider through:
  - iOS: cert/profile setup, signing config, `flutter build ipa`,
    TestFlight upload.
  - Android: keystore setup, signing config, `flutter build
    appbundle`, Play upload.
  - Web: build command, deploy to chosen target, MIME-type
    requirements.
- [ ] Key custodians named per platform (who holds the iOS
  distribution cert, who holds the Android upload key).

### Block 6 acceptance gate

- [ ] All items above ticked.
- [ ] Block 5 acceptance still green.
- [ ] Signed iOS build live on TestFlight; link in PR.
- [ ] Signed Android build live on Play Internal; link in PR.
- [ ] Web build live at chosen URL; link in PR.
- [ ] `build.md` walks an org outsider through reproducing each
  build.
- [ ] **Block 6 done. v1 shipped.**

---

## v1 exit gate

When every box above is ticked:

- [ ] All six blocks `done`.
- [ ] [THIN-SLICE.md](./THIN-SLICE.md) collapses to a one-line
  pointer at the merged PRs.
- [ ] This file (`STAGES.md`) is kept as a historical record of
  what shipped; it is **not** deleted. Future v2 stages get
  appended below as new blocks.
- [ ] [README.md](./README.md) "Status" line updated from
  "scope" to "v1 shipped."

---

## v2 placeholder

Left intentionally empty. The first PR of v2 (SDUI work) appends
its block here with the same structure: section name, item list,
acceptance gate.

The ARB-vs-ICU ADR from
[APP-SHELL §i18n](./APP-SHELL.md#i18n) lands as the very first
v2 item — the rule there is that no `lib/features/sdui/` PR
merges without that ADR cited.
