# Flutter app — load-bearing decisions

Every choice that, if reversed, would invalidate large parts of
the scope. One row per decision, with the rejected alternatives.

## What v1 is and is not

**v1 is a chassis-validation milestone.** Its success is measured
by how fast v2 (SDUI) ramps, not by what v1 itself ships. The home
screen exists to give the chassis a real consumer, not to be a
product. If a reviewer asks "why is this a defensible v1 and not a
demo?" the answer is: every load-bearing decision in this file
(drift on web, platform-branched token store, dio+retrofit, the
401 flow, the polish bar) gets exercised in production conditions
on three platforms before any IR-rendering code is written.

**Rejected: v1 includes one IR-kind renderer** (the alternative the
review surfaced). The rejection is deliberate. Pulling one renderer
forward sounds cheap but drags in the IR types, the renderer
registry shape, the transport interface, and the SDUI provider —
all of which deserve their own design pass, none of which the
home-screen-as-chassis-validator needs. The chassis decisions
above are independently load-bearing; isolating them is the
fastest path to a stable foundation.

## SQLite library: drift, not sqflite

**Choice:** `drift` (^2.28) + `drift_flutter` (^0.2.x).
**Rejected:** `sqflite` + `sqflite_common_ffi_web`.

The user's third requirement is browser support. `sqflite` has no
official web backend — community wrappers exist
(`sqflite_common_ffi_web`) but they lag the mobile API and have a
smaller user base. Drift compiles to native SQLite on iOS/Android
and to a WASM build (`sqlite3.wasm`) plus a web worker on the
browser, behind one identical Dart API.

**The cost we accept:** two files (`sqlite3.wasm` +
`drift_worker.dart.js`) must be copied into `web/` once per drift
upgrade. If they are missing or out-of-date, the web build silently
falls back to in-memory storage — data is lost on reload, and there
is no compile-time error.

**Canary mechanism (ABI-compat band, not strict equality).** A
magic-bytes check (`head -c4 sqlite3.wasm == \0asm`) only catches
"file is empty or corrupted" — it does not catch the more likely
failure mode, which is a drift upgrade that leaves the wasm/worker
pinned to an older release.

But strict version equality is *too* strict. Drift patch releases
(e.g. 2.28.0 → 2.28.1) rarely change the wasm ABI, and failing CI
on every patch bump trains people to bypass the check — which
defeats the point. The canary needs to track wasm ABI compatibility,
not the drift dependency version.

The constant in `lib/core/storage/_drift_assets_version.dart`
captures **the drift release tag the assets were downloaded from**,
plus an explicit compat band:

```dart
// The drift release tag the files in web/ came from.
// Bump when web/sqlite3.wasm or web/drift_worker.dart.js is
// refreshed from a different release.
const driftAssetsReleaseTag = '2.28.0';

// The range of drift dependency versions known to be wasm-ABI
// compatible with the assets above. Inclusive on both ends.
// Widen as new drift releases ship without wasm-side changes;
// narrow (i.e. force an asset refresh) when drift's release
// notes mention sqlite3 wasm, worker, or web schema changes.
const driftAssetsCompatRange = (min: '2.28.0', max: '2.30.99');
```

CI runs `tools/check_drift_assets.sh` which (a) parses the
resolved `drift` version from `pubspec.lock`, (b) verifies it
falls inside `driftAssetsCompatRange`, (c) verifies both
`web/sqlite3.wasm` and `web/drift_worker.dart.js` exist. The app
also asserts compat-range membership at boot in debug.

**Workflow this enables:**

- Patch/minor drift bump within the band → pubspec change only,
  CI green, assets untouched.
- Drift bump outside the band → either (a) widen the `max` after
  verifying release notes don't mention wasm/worker/web schema
  changes (one-line PR), or (b) refresh the assets and bump both
  `driftAssetsReleaseTag` and the band (three-line PR).
- Forgotten asset refresh → CI fails loudly, with a message
  pointing at the compat-range constant.

The band is intentionally narrow at v1 (one minor release worth
of headroom) because no one yet knows which drift bumps are
wasm-affecting. Block 1's implementer widens it as evidence
accumulates.

Tracked as a Block 1 acceptance criterion in [THIN-SLICE.md](./THIN-SLICE.md#block-1--scaffold--drift--web-wasm-setup).

## Auth credential storage

**Choice:** tokens, not passwords. Storage layered by platform.

| Platform | Mechanism | Backed by |
|---|---|---|
| iOS | `flutter_secure_storage` | Keychain (hardware-backed) |
| Android | `flutter_secure_storage` | Keystore + EncryptedSharedPreferences |
| Browser | **In-memory only** in a Riverpod provider | None — refresh on reload |

The raw password from the login form lives in memory for the
duration of the `POST /auth/token` call and is discarded; only the
returned bearer is stored. There is no code path that writes a
password to disk on any platform.

**Why in-memory on web** rather than `flutter_secure_storage`'s
web backend: that backend is `localStorage` + a WebCrypto-wrapped
app key. It is XSS-readable, survives tab close, and offers no
hardware backing. Storing a bearer there is no better than storing
it in plain `localStorage`. v1's posture is: on web, you re-login
on cold start. A future server-set `HttpOnly; Secure; SameSite=Lax`
refresh-token cookie can lift that without changing client code
beyond the `TokenStore` interface — that boundary is intentionally
kept thin.

**Rejected: AES-GCM-in-SQLite.** A "store the encrypted token in
the drift database, keep only the AES key in Keychain/Keystore"
scheme adds one indirection without changing the threat model.
The AES key would itself live in the platform's secure store —
which is exactly where the bearer token can live, in one step,
without writing a single line of crypto code. The drift row buys
nothing: an attacker who reads the secure-store entry reads the
key and decrypts the row; an attacker who can't read the
secure-store can't read the bearer either way. (Same conclusion
as the RN plan in
[`docs/scope/mobile/LOCAL-DB.md`](../../scope/mobile/LOCAL-DB.md#secret-handling).)

**Known Android Keystore quirk.** `flutter_secure_storage` on
Android has a recurring class of issues
([flutter-secure-storage#210](https://github.com/mogol/flutter_secure_storage/issues/210),
[#354](https://github.com/mogol/flutter_secure_storage/issues/354))
where Keystore entries are wiped on cloud-restore or after certain
OEM updates. At v1's posture ("re-login on data loss") this is
acceptable — the user hits `/login` instead of `/home` on next
launch, and the bearer is the only thing lost. It is **not**
acceptable once Block 6+ work introduces anything that depends on
local secret continuity (refresh tokens, encrypted offline cache,
biometric-gated keys). Whichever block proposes such a feature
inherits the responsibility of documenting the wipe-recovery
flow.

## REST client: dio + retrofit

**Choice:** `dio` (^5.7) + `retrofit` (^4.4) + `retrofit_generator`
(^9.x, dev).
**Rejected:** `chopper`, hand-rolled `Dio` calls, `http` package.

The user's fourth requirement is a REST client. `retrofit` gives
typed endpoint signatures via codegen on top of `dio`, which
remains the underlying transport. That matters because every
interesting cross-cutting concern (auth header injection, 401
handling, request/response logging, cancel tokens, retries) lives
in a `Dio` interceptor, and `retrofit` does not get in the way.

`chopper` is still maintained but the ecosystem has thinned —
fewer plugins, no `Dio` interop, slower release cadence. Not worth
adopting in 2026 for a new project.

Hand-rolled `Dio` is viable for fewer than ~30 endpoints. Rubix
already has more than that, and the IR-resolving endpoints will
multiply once SDUI lands in v2. Codegen pays for itself.

**Cost accepted:** `build_runner` adds 3–8s to incremental builds.
Generated `*.g.dart` and `*.freezed.dart` files **are checked in**
so CI does not need to regenerate on every run. This is the
standard Dart convention; do not `.gitignore` them.

## State management: Riverpod 3 with codegen

**Choice:** `flutter_riverpod` (^3.0) + `riverpod_annotation` (^3.0)
+ `riverpod_generator` (^3.x, dev) + `riverpod_lint` (^3.x, dev).
**Rejected:** `provider`, `bloc`, raw `InheritedWidget`.

Riverpod 3.0 (September 2025) made `@riverpod`-annotated providers
the idiomatic surface and deprecated several manual constructors.
The codegen path is the one the maintainers point new projects at;
no reason to fight that.

`bloc` is a perfectly fine alternative but doubles the amount of
boilerplate per feature and the team has no prior `bloc`
investment. `provider` is in maintenance mode and `riverpod` is
its successor by the same author.

`riverpod_lint` is **not optional** — the Dart analyzer alone
misses common provider misuse (forgot `keepAlive`, ref-leak across
async gaps, etc.). It runs in CI.

## Project layout: feature-first + core

**Choice:** `lib/core/<concern>/` + `lib/features/<feature>/{data,domain,presentation}/`.
**Rejected:** layer-first (`lib/{data,domain,presentation}/`), all-flat.

This is the prevailing 2026 Flutter pattern and matches what the
Riverpod and Code-with-Andrea reference apps converge on. It also
mirrors what the TS frontend already does with
`rubix/frontend/src/{components,routes,lib}` at the top and
feature folders below — a reviewer coming from the web app will
recognize the shape.

Generated files (`*.g.dart`, `*.freezed.dart`) sit next to the
file that owns them, **not** in a centralized `generated/` folder.
Per Dart convention.

Full tree in [APP-SHELL.md §Folder layout](./APP-SHELL.md#folder-layout).

## Monorepo integration

**Choice:** `rubix/flutter/` as a sibling Dart package, outside
the pnpm workspace. No `melos`.

**Why outside pnpm:** Pub and pnpm don't share resolution. A Dart
package whose folder contains a `node_modules/` subtree confuses
`dart analyze`, and `pnpm install` will happily descend into
folders it shouldn't. The cleanest seam is "Dart on one side, JS
on the other, no overlap."

**Why no melos:** `melos` is a Dart-workspace task runner that
shines when you have multiple Dart packages with cross-deps
(`rubix_api_client` consumed by both an app and a docs site). We
have one Flutter package. Adding `melos` now would be ceremony
without payoff.

**Concrete revisit trigger:** the first time we extract anything
from `rubix/flutter/lib/` into its own Dart package, melos comes
with the extraction PR — not before, and not as a separate
"infrastructure" PR. Tying melos adoption to the act that creates
the second package is the only trigger that gets acted on; "when
a shared library appears" is vague enough that nobody pulls the
trigger. The two-package state is what melos exists for, so the
two events ship together.

**API contract sharing.** v1 hand-mirrors a small number of DTOs
(`AuthRequest`, `AuthResponse`, the dashboard data shape) in
Dart with `freezed`. Once the IR / SDUI work begins in v2, the
project switches to OpenAPI-generated Dart DTOs via
`openapi_generator` (^5.x) consuming the same spec the TS frontend
already uses. Hand-mirroring more than ~10 DTOs is the trigger to
switch.

This trigger is intentional: starting with codegen on day one
means debugging codegen on day one, which slows the foundation
work for no benefit at this scale.

## Theme seed color

**Choice:** Material 3 `ColorScheme.fromSeed(seedColor: Color(0xFF1F2A2E))`
for light, dark derived by `Brightness.dark`. Pinned in
`lib/core/theme/app_theme.dart`.

**Provenance.** The hex value is the sRGB conversion of the web
kit's `--primary` light-mode token,
`oklch(0.218 0.008 223.9)`, sourced from
[`packages/starter-ui-kit/src/styles/globals.css`](../../../../packages/starter-ui-kit/src/styles/globals.css)
at the time of writing. The conversion was done once, by hand, and
the result is the source of truth for the Flutter app from here
on.

**Why pin instead of "track whatever the web has."** The reviewer
suggested deferring to "whatever the web has at Block 4 time."
That is exactly the underspecification we are trying to avoid.
Pinning the hex now means:

- Block 4's implementer knows the answer without a chase.
- The two apps will drift visually over a quarter if the web kit
  re-themes. That drift is **a feature, not a bug** — the
  alternative is auto-tracking a moving target, which means
  extracting a `rubix_theme` package right now to share tokens.
  That extraction is premature; see
  [Monorepo integration](#monorepo-integration) above.

**Sync mechanism.** Manual. When the web kit re-themes
intentionally and we want the Flutter app to follow, that is a PR
that bumps the constant in `app_theme.dart` and updates this
section's hex. A `rubix_theme` Dart package is the right answer
the second time we do that sync — at which point it triggers the
melos adoption above, on the same PR.

## Lint set: `very_good_analysis`, with a per-rule disable budget

**Choice:** `very_good_analysis` (^6.0) + `riverpod_lint` (^3.x).
**Rejected:** `flutter_lints` as the floor.

VGA is opinionated. Three of its rules (`prefer_single_quotes`,
`lines_longer_than_80_chars`, `public_member_api_docs`) are
calibrated for library code, not app code, and will generate
visible noise on a fresh project. Specifically:

- `prefer_single_quotes` is fine and stays on.
- `lines_longer_than_80_chars` clashes with Dart's 80-column
  formatter default vs the wider lines many Flutter widget trees
  need; disable if it generates more than ~5 warnings in the
  Block 1 codebase.
- `public_member_api_docs` is a library-author rule; app code
  has no public API surface to document. Disable.

**The policy:** Block 1's implementer **may disable specific
rules** in `analysis_options.yaml` with a one-line comment per
disable explaining why. The implementer **may not** swap to
`flutter_lints` (or any other rule set) without a follow-up PR
that updates this decision and re-runs the analyzer across the
whole codebase. The reason: a lint-set swap mid-project produces
either a huge mechanical-change PR or a silent drop in coverage,
neither of which is acceptable as a Block-1 side effect.

`riverpod_lint` is **not negotiable** at any block — the analyzer
alone misses common Riverpod misuse and that is what bites in
production.

## What we are **not** deciding here

- Choice of charting library — deferred to whenever the first
  dashboard widget needs one. `fl_chart` and `syncfusion_flutter_charts`
  are both candidates.
- Choice of animation primitive — Flutter's built-in
  `AnimationController` covers everything v1 needs. `flutter_animate`
  is the likely v2 choice for declarative chains.
- Whether to use `go_router` or a hand-rolled `Navigator 2.0`
  config — picked in [APP-SHELL §Navigation](./APP-SHELL.md#navigation).
  Recording the answer is below the bar for this file.
