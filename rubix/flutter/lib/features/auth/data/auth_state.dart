/// Auth state machine values consumed by router, interceptor, and UI.
///
/// Discriminated union, not a `String? token` — every reader has to
/// pick a branch, which is the whole point of the rewrite. The old
/// `FutureProvider<String?>` collapsed "we don't know yet", "no token
/// and won't try", and "no token but might recover" into one nullable.
library;

sealed class AuthState {
  const AuthState();
}

/// Pre-bootstrap or rebuilding (active connection changed). Treat as
/// "show a spinner" — do not assume unauthenticated yet.
class AuthUnknown extends AuthState {
  const AuthUnknown();
}

/// No token, and the controller is not going to silently issue one.
/// Reason is for diagnostics only — the UI should show a login prompt
/// or an empty state regardless of which reason fired.
class AuthUnauthenticated extends AuthState {
  const AuthUnauthenticated({this.reason});
  final String? reason;
}

/// Valid bearer token. Stable until `markExpired()` or `logout()`.
class AuthAuthenticated extends AuthState {
  const AuthAuthenticated(this.token);
  final String token;
}
