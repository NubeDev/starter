# Flutter app — app shell

The `rubix/flutter/` app is a single Flutter package targeting iOS,
Android, and the browser. It mirrors the layered shape of
[`rubix/frontend/`](../../../frontend/): chrome on top, providers
in the middle, transport at the bottom.

## Folder layout

```
rubix/flutter/
  pubspec.yaml
  analysis_options.yaml      ← very_good_analysis + riverpod_lint
  build.yaml                 ← codegen config (retrofit, drift, freezed, riverpod)
  web/
    index.html
    sqlite3.wasm             ← drift web — see DECISIONS
    drift_worker.dart.js     ← drift web — see DECISIONS
  ios/                       ← generated
  android/                   ← generated
  lib/
    main.dart                ← runApp(ProviderScope(child: RubixApp()))
    app.dart                 ← RubixApp: MaterialApp.router + theme wiring
    core/
      network/
        dio_client.dart      ← Dio instance + interceptors
        auth_interceptor.dart
        log_interceptor.dart
        api_client.dart      ← @RestApi retrofit definition
        network_providers.dart
      storage/
        app_database.dart    ← @DriftDatabase
        tables/
          connections_table.dart
          connection_state_table.dart
        daos/
          connection_dao.dart
        database_providers.dart
      auth/
        token_store.dart           ← interface
        token_store_mobile.dart    ← flutter_secure_storage impl
        token_store_web.dart       ← in-memory impl
        token_store_providers.dart ← kIsWeb branching
      theme/
        app_theme.dart       ← ThemeData light/dark
        theme_providers.dart
      i18n/
        l10n.dart            ← gen-l10n entry
        intl_providers.dart
        arb/
          app_en.arb
          app_es.arb
      router/
        app_router.dart      ← go_router config
        route_paths.dart
    features/
      connections/
        data/
          connection_repository.dart
        domain/
          connection.dart    ← freezed
        presentation/
          connections_list_screen.dart
          add_connection_screen.dart
          edit_connection_screen.dart
          connection_controller.dart   ← @riverpod
      auth/
        data/
          auth_repository.dart
          dto/
            login_request.dart   ← freezed
            login_response.dart  ← freezed
        domain/
          session.dart
        presentation/
          login_screen.dart
          login_controller.dart
      home/
        presentation/
          home_screen.dart            ← the one hardcoded dashboard
          home_controller.dart
    shared/
      widgets/
        loading_indicator.dart
        error_panel.dart
        unreachable_panel.dart
      extensions/
        async_value_x.dart
  test/
    core/
    features/
  integration_test/
    smoke_test.dart                   ← login → home, on all platforms
```

**One file, one responsibility.** No `utils.dart`, no
`primitives.dart`. Generated files (`*.g.dart`, `*.freezed.dart`,
`app_database.g.dart`) sit next to their owner and are checked into
git.

## Provider stack

`app.dart` mounts the following providers, outermost first. The
order matters — each layer reads from the ones above it.

1. **`ProviderScope`** — Riverpod root. Lives in `main.dart`.
2. **`appDatabaseProvider`** (keepAlive) — opens the drift DB,
   runs migrations, exposes the handle.
3. **`tokenStoreProvider`** — branches on `kIsWeb`. Mobile returns
   `MobileTokenStore(FlutterSecureStorage())`, web returns
   `WebTokenStore()` (in-memory).
4. **`activeConnectionProvider`** (`AsyncNotifierProvider`) — reads
   the active connection id from the `connection_state` table,
   exposes the resolved `Connection` row. Every other provider
   that depends on "which server" depends on this.
5. **`dioProvider`** — constructs the `Dio` instance from the
   active connection's `baseUrl`. Rebuilt **only** when
   `baseUrl` changes. The token is **not** a build input; the
   `AuthInterceptor` reads the current token via a `Ref`
   captured at construction, so token updates do not invalidate
   the `Dio` instance. See
   [Why the `dioProvider` does not rebuild on token change](#why-the-dioprovider-does-not-rebuild-on-token-change)
   below.
6. **`apiClientProvider`** — `ApiClient(dio)` from retrofit.
7. **`authRepositoryProvider`**, **`connectionRepositoryProvider`** —
   the only consumers of `apiClientProvider`. Features depend on
   repositories, never on `ApiClient` directly.
8. **Feature controllers** (`@riverpod`-annotated) — read the
   repositories they need, expose state to widgets.

`MaterialApp.router` sits below `ProviderScope` and reads the
`appRouterProvider`. Theme is read from `themeProvider`. Locale is
read from `localeProvider`.

**Total length of `app.dart`:** ≤60 lines. If it grows past that,
something is doing too much.

## Auth

Two-step exchange, same shape as the RN plan:

1. **Issue.** POST `{ email, password }` to
   `<active.baseUrl>/api/v1/auth/token`. On 200, store the returned
   `{ token, expires_at }` via `tokenStoreProvider`.
2. **Install.** Subsequent `dio` calls carry the token via
   `AuthInterceptor` — no per-call header plumbing.

The endpoint is the one in
[`crates/starter-auth-users/src/routes/token.rs`](../../../../crates/starter-auth-users/src/routes/token.rs).
Full contract in
[`docs/design/auth/token-issuance.md`](../auth/token-issuance.md).

**Where the token lives.** See
[DECISIONS §Auth credential storage](./DECISIONS.md#auth-credential-storage).
Quick reference:

| Platform | Cold-start behavior |
|---|---|
| iOS / Android | Token read from Keychain/Keystore; user lands on home. |
| Web | No token (in-memory only); user lands on login. |

**Refresh tokens are not in v1.** Recorded in
[NON-GOALS.md](./NON-GOALS.md#auth--security).

### 401 re-entrancy

The happy path is short ("401 → evict → /login → restore pending
route"). The interesting cases are the ones that always show up
in week three of any auth pipeline, so they get specified now.

Three concurrency hazards to handle. The interceptor implements
all three; the Block 3 implementer must not ship without test
coverage for each.

**A. 401 stampede.** Multiple in-flight requests can 401
simultaneously (e.g. expiry crossed between request issue and
response). The interceptor must guarantee:

- The token is evicted **exactly once** per stampede. Use a
  module-level `_evictionLock` (a `Future<void>?` guard): the
  first 401 sets it to a future that does the eviction work;
  subsequent 401s `await` the same future. Cleared when the
  future completes.
- The redirect to `/login` fires **exactly once**. The router's
  `refreshListenable` handles deduplication naturally because
  `tokenStoreProvider` only emits one state change for an
  eviction — but the redirect handler itself must also be
  idempotent against being asked to navigate to `/login` while
  already on `/login`.
- The `pendingRouteProvider` records the route that was
  **navigated to** when the stampede started, not whichever
  in-flight call happened to 401 last. The router writes
  `pendingRoute` on the redirect-to-`/login` transition; the
  interceptor does not write it directly.

**B. Recursive 401 from `/auth/token` itself.** If the
post-eviction re-login call (`POST /api/v1/auth/token`) returns
401 — bad credentials, deleted account, server-side principal
revocation — the interceptor must **not** treat that 401 the
same as a normal one. Otherwise the eviction-and-redirect dance
infinite-loops: re-login fails → 401 → evict (no-op, already
evicted) → redirect to `/login` (no-op, already there) → user
hits "sign in" → re-login fails → repeat.

The mechanism: the `AuthInterceptor` keeps a list of
**auth-exempt paths** that bypass its 401 handler entirely. A
401 from an exempt path passes through as a normal error for
the caller to surface (typically as an inline "wrong email or
password" message on the login form). v1's exempt paths:

```
POST /api/v1/auth/token    ← issuance; failure is a credentials error
POST /api/v1/auth/logout   ← logout; 401 means "already logged out"
GET  /healthz              ← unauth; 401 should not be possible but be defensive
```

The list lives in `core/network/auth_interceptor.dart` as a
`Set<String>` constant. Adding a future auth-exempt route is a
one-line change.

**C. Explicit logout vs interceptor-driven logout.** The two
have different `pendingRouteProvider` semantics:

| Source | `pendingRouteProvider` after |
|---|---|
| User taps "Sign out" in `/settings` | **Cleared.** User intent is "I am done"; restoring a deep link after they re-login would be surprising. Router lands on `/connections` after re-login. |
| `AuthInterceptor` 401 eviction | **Preserved.** User intent was "look at X"; the auth failure was incidental. After re-login the router restores X. |

The mechanism: `AuthRepository.logout()` clears
`pendingRouteProvider` before evicting the token; the
interceptor never touches it directly (the router sets it on the
redirect-to-`/login` transition, which only fires under
interceptor-driven eviction because explicit logout already
cleared the value).

**D. Token expiry without a 401.** v1 has no proactive refresh,
so a client-known expired token (per `expires_at`) is still sent
on the request and the server is the one that issues the 401.
This is intentional — proactive expiry handling would duplicate
the 401 path with no benefit. `expires_at` is stored alongside
the token only so a future refresh implementation has the field
without a migration.

The pending-route restoration after re-login handles the
"server returned 404 for the restored route" case the same way
as before: fall back to `/home` and toast.

## Storage

| Concern | Mechanism | Lives in |
|---|---|---|
| Saved connections (URL, label, last-used) | SQLite via drift | `connections` table |
| Active-connection id + last-opened route | SQLite via drift | `connection_state` table (single row) |
| Optional PIN gating `/connections*` | SQLite via drift | `app_settings` table (single row, `connections_pin` column) |
| Per-connection bearer token | `flutter_secure_storage` (mobile) / in-memory (web) | `token_store_*.dart` |
| Theme mode (light/dark/system) | `shared_preferences` | `theme_providers.dart` |
| Locale (en/es) | `shared_preferences` | `intl_providers.dart` |
| HTTP cache | None in v1 | — |

**Why two storage layers** (SQLite + shared_preferences): drift is
worth it for the connections list (multiple rows, queryable,
relational extensions later). It is overkill for a single enum
("light"). `shared_preferences` is one line per scalar and is the
idiomatic Flutter choice.

**On the PIN:** stored as plaintext in `app_settings.connections_pin`.
This is a casual lock against shoulder-surfing, not an auth credential
— anyone with filesystem access to the SQLite DB already has the
stored bearer token, so hashing the PIN would not raise the security
floor. The session-scoped unlock flag is held in memory in
`pinUnlockedProvider` (cleared on logout / app restart).

## Navigation

`go_router` (^14). File-equivalent route definitions live in
`lib/core/router/app_router.dart`. Paths:

```
/                          ← splash; redirects based on active connection
/connections               ← list, tap to activate, swipe to delete
/connections/new           ← add connection (URL + label + email + password + LAN scan)
/connections/unlock        ← PIN entry; only reachable when a PIN is set
/connections/:id           ← edit connection metadata, force re-login
/login                     ← login against active connection
/home                      ← the one hardcoded dashboard (v1)
/settings                  ← theme + locale + optional connections PIN
```

The redirect hook in `app_router.dart` is a pure function of
four inputs: whether an active connection exists, whether a
token is present, whether a connections PIN is set + unlocked,
and which route the user was trying to reach. Spelling it out as
a table prevents the "it works on the happy path but the wrong
screen flashes on cold start" class of bugs.

| Active conn? | Token? | PIN set + unlocked? | Requested route | Redirect to | Notes |
|:-:|:-:|:-:|---|---|---|
| no  | —   | — | any                | `/connections/new` | Cold install. Any deep link is dropped; user must add a server first. The PIN gate is intentionally skipped here — the first-run path must reach `/connections` so the user can add one. |
| yes | —   | set + locked | `/connections*` | `/connections/unlock` | PIN gate. Bypassed only when no connection exists yet (above row). |
| yes | —   | unset ∨ unlocked | `/connections/unlock` | `/connections` | User landed on the lock screen but doesn't need it; hop through. |
| yes | no  | — | `/login`           | *(no redirect)*    | Already at the right place. |
| yes | no  | — | `/connections*` or `/connections/unlock` | *(no redirect)* | Connections management is reachable without a token so the user can switch servers without logging into the broken one. |
| yes | no  | — | anything else      | `/login`           | Sets `pendingRouteProvider` to the requested route so post-login restore works. |
| yes | yes | — | `/login`           | `pendingRoute` or `/home` | Already authenticated; bounce off the login screen. |
| yes | yes | — | `/`                | `pendingRoute` or `/home` | Splash route resolves. |
| yes | yes | — | anything else      | *(no redirect)*    | Normal navigation. |
| yes | yes | — | (after re-login, restored route returns 404) | `/home` + toast | See [401 re-entrancy](#401-re-entrancy). |

`go_router`'s `refreshListenable` is wired to a thin `Listenable`
that fans in `activeConnectionProvider` + `tokenStoreProvider`
changes, so a logout from any screen triggers the redirect. The
listenable does **not** fan in route changes themselves —
`go_router` already evaluates the redirect on every navigation;
double-firing it is what causes redirect loops.

## Theme

`ThemeData.light()` and `ThemeData.dark()` constructed in
`app_theme.dart` from a Material 3 `ColorScheme.fromSeed`. The
seed color is pinned to `Color(0xFF1F2A2E)` — the sRGB conversion
of the web kit's `--primary` light-mode token. The full rationale,
provenance, and manual-sync policy are in
[DECISIONS §Theme seed color](./DECISIONS.md#theme-seed-color).

Dark/light/system follows `MediaQuery.platformBrightnessOf` when
the user picks "system" in `/settings`.

There is **no design-token package** in v1. If a second Dart
consumer ever needs the same palette, that extraction triggers
both the `rubix_theme` package *and* melos adoption — see
[DECISIONS §Monorepo integration](./DECISIONS.md#monorepo-integration).

## i18n

`flutter gen-l10n` over `lib/core/i18n/arb/app_en.arb` +
`app_es.arb`. Catalogs intentionally start tiny — only the strings
the v1 screens use. Adding a string is: add to both ARB files, run
`flutter gen-l10n`, use `AppLocalizations.of(context).<key>`.

**Reuse from the web app.** The web app's i18n catalog at
[`rubix/frontend/src/i18n/{en,es}.json`](../../../frontend/src/i18n/)
uses a different format (react-intl ICU JSON) and a different key
namespace. v1 does not try to share the catalog — the v1 string set
is small enough to retype.

**ARB-vs-ICU ADR trigger.** An ADR on whether to convert the JSON
catalog to ARB or to ingest it at build time must land **before
the first SDUI renderer ships in v2** — not "during v2 work."
SDUI renders user-authored content whose strings live in the
shared catalog the web app already consumes; deciding the catalog
format after the renderer is half-built means re-doing the
renderer's string-resolution path. The forcing function is: the
first PR that adds a `lib/features/sdui/` folder must cite this
ADR or it does not merge.

## Network

One `Dio` instance, three interceptors, in this order:

1. **`LogInterceptor`** — only in debug. Pretty-prints request and
   response. `pretty_dio_logger`.
2. **`AuthInterceptor`** — injects `Authorization: Bearer <token>`
   if a token is present; on 401, evicts the token and triggers
   re-login (see Auth above).
3. **Retrofit-generated calls** — read the configured dio.

`baseUrl` is the active connection's URL. Switching connection
re-builds the `Dio` instance via Riverpod, so a stale baseUrl is
not representable. There is **no global `Dio` singleton**; everything
goes through `dioProvider`.

User-Agent fixed to `rubix-flutter/<pubspec.version> (<platform>)`,
same convention the RN plan uses, for proxy allow-listing.

### Why the `dioProvider` does not rebuild on token change

The naive shape — "rebuild `Dio` when either baseUrl or token
changes" — is a footgun. Tokens change far more often than
baseUrls (every login, every 401-driven eviction, every future
refresh). A rebuild-on-token model causes three concrete
problems:

1. **In-flight request orphaning.** A request issued against
   `dio_v1` does not magically migrate to `dio_v2` when the
   token rotates. Either the old `Dio` is disposed mid-flight
   (request error) or it is kept alive past its useful life
   (request completes against a stale token, looks like success
   but the server response was issued for the old principal).
2. **Interceptor identity confusion.** The `AuthInterceptor`'s
   401-handler closes over a `Ref` and a `TokenStore`. If the
   interceptor instance is recreated on every token change, the
   handler that processes a 401 may belong to a `Dio` that is
   already being torn down by the rebuild. State updates from
   that handler land in the wrong place.
3. **Provider-graph thrash.** Anything downstream of
   `dioProvider` (api client, repositories, controllers) gets
   invalidated on every login. That is not what the consumers
   expect or want.

The fix is the conventional pattern: the `Dio` instance and the
`AuthInterceptor` are stable across token rotations.
`AuthInterceptor` reads the current token at request time via a
`Ref` captured at construction:

```dart
class AuthInterceptor extends Interceptor {
  AuthInterceptor(this._ref);
  final Ref _ref;

  @override
  Future<void> onRequest(RequestOptions options, RequestInterceptorHandler handler) async {
    final token = await _ref.read(tokenStoreProvider).read();
    if (token != null) options.headers['Authorization'] = 'Bearer $token';
    handler.next(options);
  }
}
```

`dioProvider` therefore depends on `activeConnectionProvider`
(for baseUrl) but **not** on `tokenStoreProvider`. Token
mutations are invisible to the provider graph; the next request
just picks up the new token via the interceptor's live read. The
only event that disposes a `Dio` is the user switching to a
different saved connection.

This is also what makes the 401 flow tractable — see
[401 re-entrancy](#401-re-entrancy) below.

## Assets

App icons, splash screens, and any bitmap assets (logo SVG, brand
glyphs) live under `assets/` at the package root, referenced via
the `flutter.assets:` block in `pubspec.yaml`. Generated platform
artifacts (`ios/Runner/Assets.xcassets/...`, `android/app/src/main/res/...`,
`web/icons/...`) are produced by the two dev-only tools below;
they are committed to git so a fresh checkout has a buildable app
without rerunning the generators.

```
rubix/flutter/
  assets/
    brand/
      logo.svg               ← source vector
      logo-mark.png          ← 1024×1024 raster, source for icons
    splash/
      splash.png             ← centered glyph on transparent bg
```

**Tooling (dev_dependencies, not runtime):**

- `flutter_launcher_icons` (^0.13.x) — reads
  `flutter_launcher_icons:` config in `pubspec.yaml`, emits
  iOS/Android/web icon sets from `logo-mark.png`. Re-run with
  `dart run flutter_launcher_icons` whenever the icon source
  changes.
- `flutter_native_splash` (^2.4.x) — reads `flutter_native_splash:`
  config in `pubspec.yaml`, emits the native splash for the three
  platforms. Re-run with `dart run flutter_native_splash:create`
  when the splash source changes.

Both tools are listed in [PACKAGES.md](./PACKAGES.md#dev-dependencies).
Neither runs on every build; they are explicit dev-time generators
whose outputs are committed.

There is **no `flutter_gen` adoption** in v1. The asset count is
small enough that hand-referenced `AssetImage('assets/brand/logo-mark.png')`
is clearer than generated `Assets.brand.logoMark`. Revisit once
the asset count crosses ~15 files.

## Env

`String.fromEnvironment` at build time, fed via
`--dart-define=RUBIX_DEFAULT_BASE_URL=...`. Optional. If set, the
first launch seeds the connections table with one entry so emulator
builds don't require manual entry. Production builds have no
default.

There is no `.env` file mechanism in v1; `flutter_dotenv` adds a
runtime file dependency that interacts badly with web release
builds. `--dart-define` is the official Flutter answer.

## Platform-specific concerns

The platform branches are confined to two places:

- `lib/core/auth/token_store_*.dart` — `kIsWeb` selects the impl.
- `web/` — `sqlite3.wasm` + `drift_worker.dart.js` for drift.

**No feature file branches on platform.** If a feature needs
to vary by platform, that branch lives behind an interface in
`core/`. The features layer stays platform-agnostic — that is the
contract that lets us claim "one codebase, three platforms."

## Testing surface (v1)

| Layer | Tool | What is covered |
|---|---|---|
| Pure Dart (DTOs, mappers) | `package:test` | freezed equality, JSON round-trip |
| Repositories | `package:test` + `mocktail` | error paths, 401 handling |
| Widgets | `flutter_test` | each screen renders against a fake controller |
| Integration | `integration_test` | login → home smoke, runs on iOS/Android/Chrome in CI |

Coverage targets are not set in v1 — chase them once the chassis
stabilizes. Block 5 of the thin slice puts a green integration
test on all three platforms as the exit gate, and that is the bar
for "v1 done."
