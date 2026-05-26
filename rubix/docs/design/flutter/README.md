# Flutter app — scope

**Goal:** ship a Flutter app (iOS, Android, browser) that is a
**multi-instance** client for `rubix-agent`. One install, many
saved backend connections. v1 validates the chassis (drift on
three platforms, platform-correct token storage, dio+retrofit,
401 handling, theme, i18n) against one hardcoded home screen;
SDUI and dashboards are v2.

**Status:** scope. Nothing scaffolded yet. Lives directly under
`docs/design/flutter/` rather than going through a `scope → design`
promotion — the React Native plan at [`docs/scope/mobile/`](../../scope/mobile/)
used that ritual and we are not reusing it for the Flutter effort.

## Why Flutter (and not the RN plan)

The RN scope at [`docs/scope/mobile/`](../../scope/mobile/) assumed
the TypeScript chassis could be reused below two seams
(`@nube/starter-ui-kit` and `@nube/starter-ui-sdui-react`). That gave
React Native a real reuse story for the IR, the SDUI registry, the
auth strategy, react-query hooks, and i18n catalogs.

Flutter does **not** get that reuse. A Flutter app talks to the same
HTTP API, but every layer above the wire is rebuilt in Dart. The
trade is: lose TS chassis reuse, gain a single native runtime
across iOS / Android / browser with one widget tree, one state
model, and one styling story.

That trade is the whole reason this scope exists as a sibling, not a
fork of the RN plan.

## Why "foundations first" — v1 as a chassis-validation milestone

The RN plan led with the SDUI thin slice — render one IR-driven
dashboard end-to-end as the first deliverable. The Flutter plan
inverts that. v1 is **explicitly a chassis-validation milestone**,
not a product milestone.

**What v1 proves:**

- Drift works identically across iOS, Android, and the browser
  (the only Dart SQL library with a credible answer on all three).
- The `kIsWeb`-branched `TokenStore` correctly gives mobile a
  Keychain/Keystore-backed bearer and web an in-memory bearer
  with no shared code path leaking the weaker guarantee upward.
- One `Dio` instance with an `AuthInterceptor` survives every
  401 path (cold-start no-token, mid-session expiry, server
  unreachable) without flaking the navigation stack.
- The look-and-feel bar (theme, i18n, polish pass) is set on the
  chassis *before* feature volume can muddy the visual baseline.

**What v1 explicitly does not prove:**

- That a user-authored dashboard renders correctly. There is no
  IR plumbing. The home screen calls `GET /api/v1/auth/me` and
  `GET /healthz` — enough to exercise auth-gated and unauth-gated
  paths, not enough to claim the platform's actual product works.

**v2 commitment.** SDUI starts the PR after v1's Block 6 merges.
The same backend SDUI route (`POST /api/v1/ui/resolve`) the web
app and the RN plan target is the v2 entry point. v1 is
foundation work whose value is measured in v2's ramp speed — if
v2's first IR-kind renderer takes more than a week of chassis
fights, v1 failed at its actual job.

Direct answer to the obvious reviewer question — "why is this a
defensible v1 and not a demo?" — is in
[DECISIONS §What v1 is and is not](./DECISIONS.md#what-v1-is-and-is-not).

See [NON-GOALS.md](./NON-GOALS.md) for what this buys us in terms
of de-scoped surface.

## The four requirements that shaped this scope

User-stated, in their words:

1. **SQLite.** Used for the multi-connection store and any cached
   read models. → `drift` (not `sqflite`), because of (3).
2. **Password storage.** Tokens — not raw passwords — live in
   `flutter_secure_storage` on mobile and in-memory on web. The
   raw password is never persisted. See
   [APP-SHELL §Auth](./APP-SHELL.md#auth).
3. **iOS + Android + browser.** Three platforms, one codebase.
   Drives the drift choice and the web-secure-storage caveat.
4. **REST client.** `dio` + `retrofit` (typed, codegen'd), with
   one shared `Dio` instance carrying an auth interceptor.

Each requirement has a section in [APP-SHELL.md](./APP-SHELL.md) and
a row in the [Decision matrix](./DECISIONS.md).

## Contents

- [DECISIONS.md](./DECISIONS.md) — the load-bearing technical choices
  (drift vs sqflite, retrofit vs chopper, riverpod 3, secure-storage
  on web), each with the rejected alternative and why.
- [APP-SHELL.md](./APP-SHELL.md) — the `rubix/flutter/` app layout:
  folder structure, provider graph, navigation, env, storage, auth.
- [PACKAGES.md](./PACKAGES.md) — the full Flutter package list with
  pinned major versions and the reason each one is in.
- [THIN-SLICE.md](./THIN-SLICE.md) — the first milestone, broken
  into ordered blocks. Six blocks; each one is a PR. Explains
  each block.
- [STAGES.md](./STAGES.md) — the same six blocks as a tickable
  item-level checklist with acceptance gates. **Tracks** each
  block; THIN-SLICE explains them. An agent doing the work
  reads THIN-SLICE for context and updates STAGES.
- [FILE-LAYOUT.md](./FILE-LAYOUT.md) — one-responsibility-per-file
  rules for Dart/Flutter. Mirrors the parent
  [`rubix/FILE-LAYOUT.md`](../../../FILE-LAYOUT.md) section-by-
  section; same 400-line ceiling, same naming taboos, Flutter-
  specific worked examples (widgets, controllers, codegen
  exemptions).
- [NON-GOALS.md](./NON-GOALS.md) — what v1 explicitly does **not**
  ship, so reviewers can reject scope creep by name.

There is no `REUSE.md` (Flutter does not import workspace
packages) and no `NEW-PACKAGES.md` (no first-party Flutter packages
yet — everything lives in the `rubix/flutter/` app until a second
consumer materializes).

## Location in the monorepo

```
rubix/
  flutter/                 ← this app, sibling to rubix/frontend/
    pubspec.yaml
    lib/...
    web/                   ← sqlite3.wasm + drift_worker.dart.js
    test/...
```

`rubix/flutter/` is **not** part of the pnpm workspace. Pub and
pnpm don't share dependency resolution, and putting a Dart package
inside a folder that has a `node_modules/` subtree confuses
`dart analyze`. CI handles it as a separate job. See
[DECISIONS §Monorepo integration](./DECISIONS.md#monorepo-integration).

## Backend prerequisites

Both already landed:

1. **`POST /auth/token`** in `starter-auth-users`
   ([`crates/starter-auth-users/src/routes/token.rs`](../../../../crates/starter-auth-users/src/routes/token.rs)):
   `{ email, password, tenant_id? } → { token, expires_at,
   token_type: "Bearer" }`. Mobile uses this for every saved
   connection.
2. **Bearer acceptance** via
   [`crates/starter-auth-users/src/principal_layer.rs`](../../../../crates/starter-auth-users/src/principal_layer.rs).

The full contract is recorded in
[`docs/design/auth/token-issuance.md`](../auth/token-issuance.md).
The Flutter app is the third client of that route (after RN and
CLI consumers).
