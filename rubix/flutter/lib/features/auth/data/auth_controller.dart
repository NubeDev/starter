import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_api/rubix_api.dart';
import 'package:rubix_flutter/core/api/api_providers.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/router/app_router/app_router.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

/// Single owner of auth state. Replaces the previous tangle of
/// `currentTokenProvider` + `authGaveUpProvider` + `_AutoLoginAttempts`
/// + `_evictionLock`.
///
/// State transitions:
/// - Active connection changes ──▶ build() runs. Existing token in
///   store wins; otherwise one silent auto-login attempt against
///   saved credentials. If that fails: `Unauthenticated(reason)`.
/// - User calls `login(...)` ──▶ explicit attempt, errors surface
///   as `AsyncError` so the UI can display them.
/// - 401 from AuthInterceptor ──▶ `markExpired()`. Single in-flight
///   reissue; collapse stampedes; never recurse.
/// - User calls `logout()` ──▶ store cleared, state →
///   `Unauthenticated('logged out')`.
///
/// Crucially: there is no provider invalidation inside the controller,
/// and no module-level singletons. The reissue lock is a per-instance
/// `Future?`, so hot restart / `ref.invalidate(authControllerProvider)`
/// resets it cleanly.
class AuthController extends AsyncNotifier<AuthState> {
  Future<void>? _reissueInFlight;

  @override
  Future<AuthState> build() async {
    // Rebuild on active-connection switch. A new connection means
    // the existing token (issued by the previous server) is irrelevant.
    final active = await ref.watch(activeConnectionProvider.future);
    if (active == null) {
      return const AuthUnauthenticated(reason: 'no active connection');
    }

    final store = ref.read(tokenStoreProvider);
    final existing = await store.read();
    if (existing != null && existing.isNotEmpty) {
      return AuthAuthenticated(existing);
    }

    final creds = await ref
        .read(connectionCredentialsStoreProvider)
        .read(active.id);
    if (creds == null) {
      return const AuthUnauthenticated(reason: 'no saved credentials');
    }

    try {
      final token = await _issueToken(
        email: creds.email,
        password: creds.password,
      );
      return AuthAuthenticated(token);
    } catch (e) {
      debugPrint('[auth] auto-login failed: $e');
      return AuthUnauthenticated(reason: 'auto-login failed: $e');
    }
  }

  /// Synchronous accessor for the request interceptor. Returns null
  /// when the controller is loading, errored, or unauthenticated.
  String? currentToken() {
    final s = state.value;
    return s is AuthAuthenticated ? s.token : null;
  }

  /// User-initiated login. Clears any prior failure and runs the full
  /// token issue + storage flow. Errors propagate as `AsyncError`.
  Future<void> login({
    required String email,
    required String password,
    String? tenantId,
  }) async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final token = await _issueToken(
        email: email,
        password: password,
        tenantId: tenantId,
      );
      return AuthAuthenticated(token);
    });
  }

  /// User-initiated logout. Clears the pending route (any 401 redirect
  /// target) and evicts the stored token.
  Future<void> logout() async {
    ref.read(pendingRouteProvider.notifier).set(null);
    await ref.read(tokenStoreProvider).clear();
    state = const AsyncData(AuthUnauthenticated(reason: 'logged out'));
  }

  /// Called by [AuthInterceptor] when a request returns 401. Tries
  /// exactly once to silently reissue a token from saved credentials;
  /// if that fails or no creds are saved, transitions to
  /// [AuthUnauthenticated]. Concurrent 401s collapse onto the same
  /// in-flight Future so a stampede only triggers one reissue.
  Future<void> markExpired() {
    final existing = _reissueInFlight;
    if (existing != null) return existing;
    final f = _reissueOnce();
    _reissueInFlight = f;
    return f.whenComplete(() {
      if (identical(_reissueInFlight, f)) _reissueInFlight = null;
    });
  }

  Future<void> _reissueOnce() async {
    await ref.read(tokenStoreProvider).clear();

    final active = ref.read(activeConnectionProvider).value;
    if (active == null) {
      state = const AsyncData(
        AuthUnauthenticated(reason: 'no active connection'),
      );
      return;
    }
    final creds = await ref
        .read(connectionCredentialsStoreProvider)
        .read(active.id);
    if (creds == null) {
      state = const AsyncData(
        AuthUnauthenticated(reason: 'token expired, no saved credentials'),
      );
      return;
    }

    try {
      final token = await _issueToken(
        email: creds.email,
        password: creds.password,
      );
      state = AsyncData(AuthAuthenticated(token));
    } catch (e) {
      debugPrint('[auth] reissue after 401 failed: $e');
      state = AsyncData(AuthUnauthenticated(reason: 'reissue failed: $e'));
    }
  }

  /// Issues a token via `POST /api/v1/auth/token` and persists it.
  /// Marks the active connection as used on success.
  ///
  /// Throws `StateError` if there is no active connection, or if the
  /// server returned a 2xx with an empty body (typically a misconfigured
  /// baseUrl returning the SPA index.html — the generated client then
  /// produces an empty model, and a silent empty token would loop the
  /// caller).
  Future<String> _issueToken({
    required String email,
    required String password,
    String? tenantId,
  }) async {
    final api = ref.read(apiClientProvider);
    if (api == null) throw StateError('no active connection');

    final request = TokenRequest((b) {
      b
        ..email = email
        ..password = password
        ..tenantId = tenantId;
    });
    final response = await api.getAuthApi().issueToken(tokenRequest: request);
    final body = response.data;
    if (body == null || body.token.isEmpty) {
      throw StateError(
        'empty /auth/token response — baseUrl likely points to a non-rubix server',
      );
    }

    await ref.read(tokenStoreProvider).write(
          body.token,
          expiresAt: body.expiresAt,
        );

    final repo = ref.read(connectionRepositoryProvider);
    final activeId = await repo.getActiveId();
    if (activeId != null) await repo.markUsed(activeId);

    return body.token;
  }
}

final authControllerProvider =
    AsyncNotifierProvider<AuthController, AuthState>(AuthController.new);
