import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/features/auth/data/auth_repository/auth_repository.dart';

/// Paths where 401 is a credentials error, not a session expiry.
const authExemptPaths = <String>{
  '/api/v1/auth/token',
  '/api/v1/auth/logout',
  '/healthz',
};

/// Interceptor that injects bearer tokens and handles 401 eviction.
class AuthInterceptor extends Interceptor {
  AuthInterceptor(this._ref);

  final Ref _ref;

  /// Module-level lock so stampeding 401s evict exactly once.
  static Completer<void>? _evictionLock;

  TokenStore get _store => _ref.read(tokenStoreProvider);

  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    final token = await _store.read();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  @override
  Future<void> onError(
    DioException err,
    ErrorInterceptorHandler handler,
  ) async {
    if (err.response?.statusCode != 401) {
      return handler.next(err);
    }

    // Check if this path is auth-exempt (e.g. /auth/token itself).
    final path = err.requestOptions.path;
    if (authExemptPaths.any(path.endsWith)) {
      return handler.next(err);
    }

    // Stampede-safe eviction: first 401 creates the lock, others await it.
    if (_evictionLock != null) {
      await _evictionLock!.future;
    } else {
      final completer = Completer<void>();
      _evictionLock = completer;
      try {
        await _store.clear();
        // Trip the circuit breaker BEFORE invalidating tokenStore so the
        // currentTokenProvider rebuild doesn't immediately auto-relogin
        // (which would loop forever if the new token also 401s — e.g.
        // when the server's /auth/me only accepts cookies, not bearer).
        _ref.read(authGaveUpProvider.notifier).set(true);
        _ref.invalidate(tokenStoreProvider);
      } finally {
        completer.complete();
        _evictionLock = null;
      }
    }

    handler.next(err);
  }
}
