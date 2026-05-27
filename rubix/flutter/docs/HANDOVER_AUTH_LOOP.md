# Handover — `/api/v1/auth/me` 401 reissue loop

## Symptom

After `make start-web`, the Flutter web client floods the rubix-agent
backend with `GET /api/v1/auth/me` requests. Every one returns
`401 Unauthorized` within ~6–30 ms. The Dart VM eventually dies under
load (`The Dart compiler exited unexpectedly.`) and the log is too
spammy to copy/paste out.

```
GET http://192.168.10.77:8088/api/v1/auth/me  → 401  (×∞)
```

The 401 storm starts immediately after the user activates a connection
and a token is issued. The very first `/auth/token` call succeeds — the
token is valid for other endpoints (verified by the fact that reissues
keep succeeding) — but `/auth/me` rejects it.

## Root cause

The backend's `/api/v1/auth/me` route returns 401 to a perfectly valid
bearer token. Best guess: the route was written to read a session
cookie and never wired up to honour the `Authorization` header. This
was suspected by the previous author — see the (now-deleted) comment
in the old `AuthInterceptor`:

```
// …would loop forever if the new token also 401s — e.g.
// when the server's /auth/me only accepts cookies, not bearer.
```

The old code dodged the loop with a *circuit breaker*
(`authGaveUpProvider`): on the *first* 401, trip a session-scoped flag
that suppresses all future auto-relogins. That was crude — once tripped,
nothing would re-login until the user manually re-activated a
connection — but it stopped the storm.

I rewrote the auth flow into a clean state-machine
(`AuthController`, `auth_state.dart`) and removed the breaker because
it overlapped with the per-instance reissue lock. **That removed the
brake on this specific bug.** Flow today:

1. Screen calls `/auth/me`
2. Backend → 401
3. `AuthInterceptor.onError` → `controller.markExpired()`
4. `markExpired()` → `_reissueOnce()` → `/auth/token` → 200, new token
5. State transitions to `AuthAuthenticated(newToken)`
6. `apiClientProvider` (or the screen watching auth state) retries `/auth/me`
7. Backend → 401 again (because the bug is in `/auth/me`, not in the token)
8. Goto 3.

`_reissueInFlight` collapses *concurrent* stampedes onto one reissue,
but it does *not* prevent **sequential** loops — each iteration
completes the future, clears the lock, and the next 401 starts a fresh
one. That's why the loop is steady-state, not a thundering herd.

## Where to look

- `lib/features/auth/data/auth_controller.dart` — `markExpired()` and
  `_reissueOnce()`. The fix likely lives here.
- `lib/core/network/auth_interceptor/auth_interceptor.dart` — currently
  unconditionally calls `markExpired()` on any non-exempt 401.
- `lib/features/home/presentation/home_controller.dart` — the most
  likely caller of `/auth/me`; confirm with `grep -rn 'auth/me' lib`.
  Whatever consumes `apiClientProvider` here is the retry driver.
- Backend route handler for `/api/v1/auth/me` —
  `rubix/crates/rubix-agent/src/...` — to confirm whether the 401 is
  intentional (cookies only) or a bug. **Until that is answered, the
  Flutter side has to assume `/auth/me` will keep 401'ing even with a
  fresh token.**

## Fix strategies (pick one, ideally in this order)

### A. Backend fix — `/auth/me` honours `Authorization: Bearer …`

Cleanest. If the backend is supposed to accept bearer tokens
everywhere except `/auth/token` and `/auth/logout` (it does, per
`authExemptPaths`), `/auth/me` is the outlier and should be fixed.
Verify with a curl that grabs a fresh token and hits `/auth/me`:

```bash
TOKEN=$(curl -sS -X POST http://192.168.10.77:8088/api/v1/auth/token \
  -H 'content-type: application/json' \
  -d '{"email":"...","password":"..."}' | jq -r .token)
curl -i -H "Authorization: Bearer $TOKEN" http://192.168.10.77:8088/api/v1/auth/me
```

If that returns 200, the loop will fix itself. If it returns 401, the
backend route is the bug — fix it there.

### B. Flutter side: cap reissues per token (recommended fallback)

If the backend cannot be touched, teach `AuthController` to detect
"the freshly-issued token also gets 401'd on this path." Add a small
LRU keyed by `(tokenValueHash, requestPath)` recording 401s seen since
the last successful issue. On the second 401 for the same pair within
a short window, **do not reissue** — transition straight to
`AuthUnauthenticated(reason: 'endemic 401 on $path')`.

This avoids the brittleness of the old circuit breaker (which was
session-scoped and never reset) by being scoped to the specific token
that proved bad. A successful response on any other path clears the
record.

Sketch:

```dart
final _badPathPerToken = <String, Set<String>>{};

Future<void> markExpired({String? path}) async {
  final tokenKey = sha256Short(currentToken() ?? '');
  if (path != null &&
      (_badPathPerToken[tokenKey] ?? const {}).contains(path)) {
    // Same token already 401'd here. Do not reissue.
    state = AsyncData(AuthUnauthenticated(
      reason: 'endemic 401 on $path — backend not honouring bearer?',
    ));
    return;
  }
  if (path != null) {
    _badPathPerToken.putIfAbsent(tokenKey, () => <String>{}).add(path);
  }
  // …existing _reissueOnce logic…
}
```

Then in `AuthInterceptor.onError` pass `err.requestOptions.path` into
`markExpired(path: …)`.

### C. Exempt `/auth/me` from the auth interceptor (quick patch, not a fix)

Add `'/api/v1/auth/me'` to `authExemptPaths`. The loop dies because
401 on `/auth/me` no longer triggers `markExpired()`. **Don't do this
without B as well** — a future `/auth/whatever` route with the same
bug would just resurrect the loop.

### D. Stop calling `/auth/me` (workaround, not a fix)

Find the caller (`grep -rn 'auth/me\|getMe\|/me' lib`), see what it
needs, and replace with `/auth/token` claims or a different endpoint
if the data is available there. Cuts the trigger but leaves the
underlying fragility.

## Test plan once a fix lands

1. `make start-web` — must not flood the logs. A single failed
   `/auth/me` followed by `AuthState` transitioning to
   `Unauthenticated(reason: 'endemic 401 on /api/v1/auth/me …')` is
   the expected shape if you go with strategy B.
2. Existing unit suite: `flutter test` — all 13 must still pass.
3. Add a unit test in `test/features/auth/auth_controller_test.dart`
   that drives a fake `apiClientProvider` returning a token, then a
   simulated repeated 401 on a non-exempt path, and asserts
   `markExpired()` only reissues *once*.
4. Manually open the dashboard screen, confirm the spinner resolves
   into "Not signed in: endemic 401 on …" and the retry button is
   live.

## What is **not** broken (don't waste time here)

- The state machine in `AuthController` itself. `build()`, `login()`,
  `logout()` work correctly and are covered by the existing test
  suite. The loop is purely in the 401-recovery path.
- `tokenStoreProvider` / conditional web-vs-mobile split. The token
  *is* being written and re-read correctly across reloads.
- The new `rubix_server` (REST backend for the connections/settings
  table) — that's a separate concern. Its bearer token is a different
  shared secret from the rubix-agent JWT and is unrelated to this
  loop.

## Files touched in the rewrite that landed this regression

- `lib/features/auth/data/auth_state.dart` *(new)*
- `lib/features/auth/data/auth_controller.dart` *(new)*
- `lib/features/auth/data/auth_repository/auth_repository.dart`
  *(now a re-export shim)*
- `lib/core/network/auth_interceptor/auth_interceptor.dart` *(rewritten)*
- `lib/core/router/app_router/app_router.dart` *(one-line swap)*
- `lib/features/connections/presentation/connections_list/connections_controller.dart` *(removed breaker resets in `activate()`)*
- `lib/features/sdui/presentation/dashboard_list_screen.dart` *(reads `AuthState` instead of `String? token`)*
- `lib/core/auth/token_store/token_store_providers.dart` + new
  `token_store_web_stub.dart` (unrelated VM-test fix for
  `dart:js_interop`)

## Quick triage commands

```bash
# Who calls /auth/me?
grep -rn 'auth/me\|getMe\|/me\b' rubix/flutter/lib --include='*.dart'

# Does the backend route exist?
grep -rn '/auth/me\|auth_me\|\"/me\"' rubix/crates/rubix-agent/src

# Confirm the 401 with a fresh token from outside the app
TOKEN=$(curl -sS -X POST http://192.168.10.77:8088/api/v1/auth/token \
  -H 'content-type: application/json' \
  -d '{"email":"YOUR_EMAIL","password":"YOUR_PW"}' | jq -r .token)
echo "$TOKEN"
curl -i -H "Authorization: Bearer $TOKEN" \
  http://192.168.10.77:8088/api/v1/auth/me
```

If that last curl returns 401, the bug is in the backend and strategy
A is the right answer. If it returns 200, the Flutter side has a stale
token caching bug somewhere — pivot to inspecting the interceptor's
`currentToken()` reads.
