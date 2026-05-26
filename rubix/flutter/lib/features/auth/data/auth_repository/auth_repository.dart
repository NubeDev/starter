import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_api/rubix_api.dart';
import 'package:rubix_flutter/core/api/api_providers.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/router/app_router/app_router.dart';
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

    // Notify token-dependent listeners.
    _ref.invalidate(currentTokenProvider);
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

/// Provides the current token as a future — used by the router to decide
/// redirect.
final currentTokenProvider = FutureProvider<String?>((ref) async {
  final store = ref.watch(tokenStoreProvider);
  return store.read();
});

final authRepositoryProvider = Provider<AuthRepository>((ref) {
  return AuthRepository(ref);
});
