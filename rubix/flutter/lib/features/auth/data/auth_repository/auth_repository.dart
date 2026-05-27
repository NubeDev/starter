import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_api/rubix_api.dart';
import 'package:rubix_flutter/core/api/api_providers.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/router/app_router/app_router.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

class AuthRepository {
  AuthRepository(this._ref);

  final Ref _ref;

  /// Issue a token and install it in the store via the generated
  /// `rubix_api` Dio client (`POST /api/v1/auth/token`).
  Future<void> login({
    required String email,
    required String password,
    String? tenantId,
  }) async {
    _ref.read(authGaveUpProvider.notifier).set(false);
    // User-initiated login resets the auto-login backoff counter for
    // every (connection,baseUrl) — the user may have fixed a typo'd URL.
    _AutoLoginAttempts.instance._attempts.clear();
    await _loginInternal(email: email, password: password, tenantId: tenantId);
    _ref.invalidate(currentTokenProvider);
  }

  /// Same as [login] but does not invalidate [currentTokenProvider].
  /// Used from inside that provider's auto-login path to avoid recursion.
  Future<void> loginWithoutInvalidate({
    required String email,
    required String password,
    String? tenantId,
  }) =>
      _loginInternal(email: email, password: password, tenantId: tenantId);

  Future<void> _loginInternal({
    required String email,
    required String password,
    String? tenantId,
  }) async {
    final api = _ref.read(apiClientProvider);
    if (api == null) throw StateError('No active connection');

    final request = TokenRequest((b) {
      b
        ..email = email
        ..password = password
        ..tenantId = tenantId;
    });

    final response = await api.getAuthApi().issueToken(
          tokenRequest: request,
        );
    final tokenResponse = response.data;
    if (tokenResponse == null) {
      throw StateError('Empty /auth/token response');
    }

    final store = _ref.read(tokenStoreProvider);
    await store.write(
      tokenResponse.token,
      expiresAt: tokenResponse.expiresAt,
    );

    // Mark connection as used.
    final repo = _ref.read(connectionRepositoryProvider);
    final activeId = await repo.getActiveId();
    if (activeId != null) {
      await repo.markUsed(activeId);
    }
  }

  /// Explicit logout — clears pending route before evicting token.
  Future<void> logout() async {
    // Clear pending route (user-initiated, not a 401).
    _ref.read(pendingRouteProvider.notifier).set(null);

    final store = _ref.read(tokenStoreProvider);
    await store.clear();
    _ref.invalidate(currentTokenProvider);
  }
}

/// Session-scoped circuit breaker. Set by the AuthInterceptor when a
/// freshly-issued bearer is rejected (401) — prevents [currentTokenProvider]
/// from immediately re-issuing and looping. Cleared when the user explicitly
/// activates a connection or calls [AuthRepository.login].
class AuthGaveUp extends Notifier<bool> {
  @override
  bool build() => false;

  // ignore: avoid_positional_boolean_parameters, use_setters_to_change_properties
  void set(bool value) => state = value;
}

final authGaveUpProvider = NotifierProvider<AuthGaveUp, bool>(AuthGaveUp.new);

/// Provides the current token as a future — used by the router to decide
/// redirect. If the store is empty but the active connection has stored
/// credentials, transparently re-issue a token. Returns null if there is
/// no active connection, no saved creds, the auto-login attempt fails, or
/// the circuit breaker has tripped (last token was rejected).
final currentTokenProvider = FutureProvider<String?>((ref) async {
  final store = ref.watch(tokenStoreProvider);
  final existing = await store.read();
  if (existing != null) return existing;

  // Circuit breaker: don't auto-relogin if the last token was rejected.
  if (ref.read(authGaveUpProvider)) return null;

  final active = await ref.watch(activeConnectionProvider.future);
  if (active == null) return null;

  // Hard backoff: track failed auto-login attempts per (connection,baseUrl).
  // Without this, a bogus baseUrl like "aa" causes /auth/token to return
  // the dev-server's index.html with status 200 → token parse fails →
  // store stays empty → provider re-runs → another /auth/token → infinite
  // loop. Cap at 3 attempts within a 30s window, then trip the breaker.
  final attemptKey = '${active.id}|${active.baseUrl}';
  final attempt = _AutoLoginAttempts.instance.next(attemptKey);
  if (attempt > 3) {
    // ignore: avoid_print
    print(
      '[auth] auto-login attempt cap reached for $attemptKey '
      '(attempt=$attempt) — tripping circuit breaker',
    );
    ref.read(authGaveUpProvider.notifier).set(true);
    return null;
  }

  final creds = await ref
      .read(connectionCredentialsStoreProvider)
      .read(active.id);
  if (creds == null) return null;

  try {
    await ref.read(authRepositoryProvider).loginWithoutInvalidate(
          email: creds.email,
          password: creds.password,
        );
  } catch (e) {
    // ignore: avoid_print
    print('[auth] auto-login failed (attempt=$attempt): $e');
    return null;
  }
  final token = await store.read();
  if (token == null || token.isEmpty) {
    // Login "succeeded" but no token landed — likely the server returned
    // HTML (bogus baseUrl) and the generated client silently produced an
    // empty model. Treat as failure so we backoff and trip eventually.
    // ignore: avoid_print
    print(
      '[auth] auto-login returned empty token (attempt=$attempt) — '
      'baseUrl may be wrong: ${active.baseUrl}',
    );
    return null;
  }
  // Success — reset the counter so a subsequent legitimate 401 eviction
  // gets a fresh 3-attempt budget.
  _AutoLoginAttempts.instance.reset(attemptKey);
  return token;
});

/// Per-(connection,baseUrl) auto-login attempt counter with a sliding
/// window. Single in-memory singleton — survives provider invalidations
/// (unlike a Notifier, which would reset on `invalidate(tokenStoreProvider)`).
class _AutoLoginAttempts {
  _AutoLoginAttempts._();
  static final instance = _AutoLoginAttempts._();

  static const _window = Duration(seconds: 30);
  final Map<String, List<DateTime>> _attempts = {};

  int next(String key) {
    final now = DateTime.now();
    final list = _attempts.putIfAbsent(key, () => <DateTime>[])
      ..removeWhere((t) => now.difference(t) > _window)
      ..add(now);
    return list.length;
  }

  void reset(String key) => _attempts.remove(key);
}

final authRepositoryProvider = Provider<AuthRepository>((ref) {
  return AuthRepository(ref);
});
