# Flutter app — non-goals

Things v1 explicitly does **not** ship. Listed by name so reviewers
can reject scope creep with a one-line link to this file.

## Surfaces (vs `rubix/frontend`)

| Excluded surface | Where it lives on web | Reason |
|---|---|---|
| Flow editor | [`rubix/frontend/src/routes/flows/`](../../../frontend/src/routes/flows/) | Pan/zoom node graph; not a phone-shaped interaction. |
| Warehouse admin | [`rubix/frontend/src/components/admin/warehouse/`](../../../frontend/src/components/admin/warehouse/) | Data-heavy admin; operators use the web app. |
| Extensions admin | [`rubix/frontend/src/routes/extensions.tsx`](../../../frontend/src/routes/extensions.tsx) | Same. |
| Tenant / authz admin | [`@nube/starter-ui-authz`](../../../../packages/starter-ui-authz/) | Same. |
| PDF export | [`@nube/starter-ui-export`](../../../../packages/starter-ui-export/) | DOM-only stack; if Flutter needs export, that's a separate ADR. |

## SDUI

- **No IR rendering in v1.** The home screen is hand-written. SDUI
  is a v2 candidate; the user explicitly asked to "focus on the
  foundations and look before adding lots of features."
- **No translation of `starter-ui-sdui-react` to Dart** before v2.
  Doing it now would lock in design decisions made for the TS
  chassis (renderer registry shape, transport interface) without
  any Flutter consumer pushing back on them.

## Auth + security

- **No biometric unlock.** A bearer in Keychain/Keystore is the
  baseline; biometrics is a follow-up.
- **No refresh tokens.** On 401 the app routes to login. Refresh
  is a separate auth-crate workstream that benefits web and
  mobile together; not a Flutter-only concern.
- **No raw password storage, ever.** The login form holds the
  password in memory for the duration of one HTTP call. There is
  no code path that writes a password to disk on any platform.
  This is a non-goal in the sense that "store the user's password
  for convenience" must be rejected if proposed.
- **`flutter_secure_storage` web backend is not used.** It is
  `localStorage` + a generated key — XSS-readable, not hardware
  backed. Web's bearer is in-memory only; user re-logs on cold
  start. See [DECISIONS](./DECISIONS.md#auth-credential-storage).
- **Tab refresh on web logs the user out.** Accepted v1 UX gap,
  direct consequence of the in-memory web bearer above. New tabs
  and duplicated tabs start unauthenticated for the same reason —
  there is no shared storage to inherit from. In-progress form
  state on the login or `/connections/new` screens is also lost
  on refresh. The user lands at `/login` against the active
  connection (not at `/connections/new`), so re-entering
  credentials is the only step. Fixing this needs a server-set
  `HttpOnly; Secure; SameSite=Lax` refresh-token cookie — a
  backend workstream tracked under
  [Out of scope for v1 but explicit v2 candidates](#out-of-scope-for-v1-but-explicit-v2-candidates).
- **No self-signed TLS / cert pinning.** Same posture as the RN
  plan: operators with home-LAN agents are expected to front them
  with Tailscale, a real DNS name, or a Cloudflare tunnel. v1
  trusts the system root store only.

## Storage

- **No offline cache.** REST responses are not persisted between
  launches. Once there is evidence of which pages operators want
  on flaky networks, `dio_cache_interceptor` is the likely answer.
- **No drift schema beyond `connections` + `connection_state`.**
  Anything cached lives behind a future schema migration; v1's
  schema is intentionally minimal.

## Operations

- **No code push.** Flutter does not have a sanctioned OTA
  mechanism comparable to Expo Updates. Releases go through the
  store review pipeline so we measure the real-world boot path.
- **No push notifications.** APNs / FCM is a separate workstream.
- **No deep links to specific pages.** The home screen is the
  only destination v1 routes to after login.
- **No analytics / telemetry SDK.** If/when one lands, it gets
  its own ADR (consent UX, GDPR/CCPA posture, payload audit). Not
  a "drop in Firebase Analytics" decision.

## Process

- **No `// TODO: v2` strings in code.** Per the project's standing
  rule (see the RN plan's
  [NON-GOALS §Process](../../scope/mobile/NON-GOALS.md#process)),
  comments describe the code as it is now. Deferred surfaces are
  recorded **here**, not as inline TODOs.
- **No platform branches inside features.** Platform-specific
  code lives behind a `core/` interface. A feature that imports
  `dart:io` directly fails review. `core/` itself is **exempt by
  design** — that is where the `kIsWeb` branches live
  (`core/auth/token_store_*.dart` is the canonical example). The
  rule's intent is to stop platform divergence from leaking
  upward into features, not to ban `kIsWeb` from the codebase.
- **No bypassing `flutter analyze`.** `// ignore: <lint>` is a PR
  blocker; fix the underlying issue.
- **No checked-in `.env`-style files.** `--dart-define` is the
  Flutter-blessed mechanism.

## Out of scope for v1 but explicit v2 candidates

These are not "no, never." They are "no, not now, and the trigger
is documented."

| Capability | v2 trigger |
|---|---|
| SDUI rendering | Block 5 of v1 ships and the chassis is stable. |
| OpenAPI-generated DTOs | Hand-mirrored DTO count exceeds ~10 (likely concurrent with SDUI). |
| Bespoke dashboard widgets (metric card, sparkline, etc.) | A page exists that the hand-written `HomeScreen` cannot represent. |
| Refresh tokens | Auth-crate ships the route. |
| Web `HttpOnly` refresh cookie | Same. Lets web get parity with mobile on cold-start UX. |
| Push notifications | Product asks for them with a concrete use case. |
| Offline cache | Operator evidence of flaky-network usage. |
| Biometric unlock | Demand from at least one design partner. |
| Deep links | Multiple dashboards exist (i.e. v2+ surface). |

Each of these gets its own design doc when its trigger fires —
not a paragraph appended to the present-tense files in this
folder.
