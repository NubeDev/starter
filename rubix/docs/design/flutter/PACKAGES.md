# Flutter app — packages

The full `pubspec.yaml` dependency list, pinned to major versions
as of May 2026. Every entry has a one-line reason it is in. If a
package is not here, the app does not depend on it.

## SDK constraint

```yaml
environment:
  sdk: ">=3.7.0 <4.0.0"
  flutter: ">=3.32.0"
```

Dart 3.7 and Flutter 3.32 are the floor — Riverpod 3.0 and the
current drift release require them.

## Dependencies

```yaml
dependencies:
  flutter:
    sdk: flutter
  flutter_localizations:
    sdk: flutter

  # State + DI
  flutter_riverpod: ^3.0.0          # state, DI, async caching
  riverpod_annotation: ^3.0.0       # @riverpod source for codegen

  # Navigation
  go_router: ^14.0.0                # declarative routing, deep link friendly

  # Network
  dio: ^5.7.0                       # HTTP transport + interceptors
  retrofit: ^4.4.0                  # typed REST client on top of dio
  pretty_dio_logger: ^1.4.0         # readable dev logs

  # Models
  freezed_annotation: ^2.4.4        # immutable data classes
  json_annotation: ^4.9.0           # JSON (de)serialization annotations

  # Database — drift, NOT sqflite (drives the web target)
  drift: ^2.28.0
  drift_flutter: ^0.2.4             # platform-aware file/web opener
  sqlite3_flutter_libs: ^0.5.0      # bundled SQLite for iOS/Android

  # Secure credential storage (mobile only; see DECISIONS)
  flutter_secure_storage: ^9.2.0

  # Non-sensitive prefs
  shared_preferences: ^2.3.0

  # Platform metadata
  package_info_plus: ^8.0.0         # sole use: User-Agent string in dio_client.dart
  intl: ^0.20.0                     # date/number formatting; floor matches flutter_localizations on Flutter 3.32
```

## Dev dependencies

```yaml
dev_dependencies:
  flutter_test:
    sdk: flutter
  integration_test:
    sdk: flutter

  # Lints
  very_good_analysis: ^6.0.0        # opinionated lint set
  riverpod_lint: ^3.0.0             # provider misuse rules

  # Codegen
  build_runner: ^2.4.13
  freezed: ^2.5.0
  json_serializable: ^6.8.0
  retrofit_generator: ^9.0.0
  drift_dev: ^2.28.0
  riverpod_generator: ^3.0.0

  # Testing
  mocktail: ^1.0.0                  # repository + interceptor tests

  # Asset generators (run-once tools, not runtime deps)
  flutter_launcher_icons: ^0.13.0   # icon set from assets/brand/logo-mark.png
  flutter_native_splash: ^2.4.0     # native splash from assets/splash/splash.png

  # Tooling (deferred to v2)
  # openapi_generator: ^5.0.0       # adopt when DTO count > ~10, see DECISIONS
  # openapi_generator_annotations: ^5.0.0
```

## Why not …

| Package | Reason excluded |
|---|---|
| `sqflite` | No first-class web support; defeats requirement #3. |
| `chopper` | Shrinking ecosystem, no dio interop. |
| `http` | Fine for one-off calls; missing interceptors and cancel tokens that retrofit + dio give us for free. |
| `provider` | Maintenance mode; riverpod is the successor. |
| `bloc` | Doubles per-feature boilerplate; no team prior art. |
| `auto_route` | go_router covers v1; auto_route's codegen is more useful at >20 routes. |
| `hive` / `isar` / `realm` | We need SQL, not a NoSQL or document store. The history mart side of Rubix lives in ClickHouse; the device-side store mirrors the relational shape. |
| `flutter_dotenv` | Runtime file dependency; conflicts with web release builds. Use `--dart-define`. |
| `melos` | Single Dart package; ceremony without payoff. See [DECISIONS](./DECISIONS.md#monorepo-integration). |
| `flutter_bloc` | See `bloc`. |
| `get` / `getx` | Pervasive global state, opinionated routing, opinionated DI — conflicts with every other choice on this list. |

## Web build assets

Two files must exist under `web/` and be on a release compatible
with the installed drift version:

```
web/
  sqlite3.wasm
  drift_worker.dart.js
```

Source: <https://github.com/simolus3/drift/releases>. They are
downloaded once, checked into git, and refreshed only when the
resolved `drift` version falls outside the wasm-ABI compat band
declared in `lib/core/storage/_drift_assets_version.dart`. CI
fails the web build on band-miss or missing asset files. Full
rationale in
[DECISIONS §SQLite library](./DECISIONS.md#sqlite-library-drift-not-sqflite);
CI step itself in
[THIN-SLICE Block 1](./THIN-SLICE.md#block-1--scaffold--drift--web-wasm-setup).

## Codegen targets

Run `dart run build_runner build --delete-conflicting-outputs`
when any of these change:

- `*.freezed.dart` ← `freezed_annotation`
- `*.g.dart` (JSON) ← `json_serializable`
- `*.g.dart` (retrofit) ← `retrofit_generator`
- `*.g.dart` (riverpod) ← `riverpod_generator`
- `app_database.g.dart` ← `drift_dev`

`build.yaml` configures one `builders:` section so a single
build_runner pass covers all of them. CI runs this before
`flutter analyze` and `flutter test`. Generated files **are
committed** so a fresh checkout can analyze without a codegen step.
