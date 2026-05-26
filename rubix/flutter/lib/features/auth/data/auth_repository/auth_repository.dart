import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/core/router/app_router/app_router.dart';
import 'package:rubix_flutter/features/auth/data/dto/login_request/login_request.dart';
import 'package:rubix_flutter/features/auth/data/dto/login_response/login_response.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

class AuthRepository {
  AuthRepository(this._ref);

  final Ref _ref;

  /// Issue a token and install it in the store.
  Future<void> login({
    required String email,
    required String password,
    String? tenantId,
  }) async {
    final dio = _ref.read(dioProvider);
    if (dio == null) throw StateError('No active connection');

    final request = LoginRequest(
      email: email,
      password: password,
      tenantId: tenantId,
    );

    final response = await dio.post<Map<String, dynamic>>(
      '/api/v1/auth/token',
      data: request.toJson(),
    );

    final loginResponse = LoginResponse.fromJson(response.data!);
    final store = _ref.read(tokenStoreProvider);
    await store.write(loginResponse.token, expiresAt: loginResponse.expiresAt);

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
