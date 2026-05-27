import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';

/// Paths where 401 is a credentials error, not a session expiry —
/// they must NOT trigger the auth state machine's expiry handler or
/// we'd recurse forever.
const authExemptPaths = <String>{
  '/api/v1/auth/token',
  '/api/v1/auth/logout',
  '/healthz',
};

/// Dio interceptor wired to [AuthController].
///
/// - onRequest: pulls the current token synchronously from the
///   controller. If the controller is still booting, the request
///   goes out without an `Authorization` header — the server will
///   401, which is exactly the path that recovers below.
/// - onError: on 401, asks the controller to expire-and-reissue.
///   The lock + single-attempt policy lives inside the controller,
///   so this class stays dumb.
class AuthInterceptor extends Interceptor {
  AuthInterceptor(this._ref);

  final Ref _ref;

  @override
  void onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) {
    final token = _ref.read(authControllerProvider.notifier).currentToken();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  @override
  void onError(
    DioException err,
    ErrorInterceptorHandler handler,
  ) {
    if (err.response?.statusCode != 401) {
      return handler.next(err);
    }
    if (authExemptPaths.any(err.requestOptions.path.endsWith)) {
      return handler.next(err);
    }
    // Fire-and-forget. The caller still sees the 401; by their next
    // attempt the controller will either hold a fresh token or have
    // transitioned to AuthUnauthenticated.
    unawaited(_ref.read(authControllerProvider.notifier).markExpired());
    handler.next(err);
  }
}
