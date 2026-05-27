import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_flutter/core/network/auth_interceptor/auth_interceptor.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';

/// Spy AuthController — bypasses bootstrap entirely; the interceptor's
/// contract is just "read currentToken on request, call markExpired
/// on a non-exempt 401". Everything else (reissue lock, single-attempt
/// policy) is now controller-side and covered separately.
class _FakeAuthController extends AuthController {
  _FakeAuthController({this.initialToken});

  final String? initialToken;
  int markExpiredCalls = 0;

  @override
  Future<AuthState> build() async {
    return initialToken == null
        ? const AuthUnauthenticated()
        : AuthAuthenticated(initialToken!);
  }

  @override
  String? currentToken() => initialToken;

  @override
  Future<void> markExpired() async {
    markExpiredCalls++;
  }
}

({
  AuthInterceptor interceptor,
  _FakeAuthController controller,
  ProviderContainer container,
}) _setup({String? token}) {
  final fake = _FakeAuthController(initialToken: token);
  final container = ProviderContainer(
    overrides: [
      authControllerProvider.overrideWith(() => fake),
    ],
  );
  // Warm up the notifier so `currentToken()` reads its build() result.
  container.read(authControllerProvider);
  late AuthInterceptor interceptor;
  container.read(Provider((ref) {
    interceptor = AuthInterceptor(ref);
    return null;
  }));
  return (interceptor: interceptor, controller: fake, container: container);
}

void main() {
  test('bearer header injected when controller reports a token',
      () async {
    final (:interceptor, controller: _, :container) =
        _setup(token: 'test-token-123');
    addTearDown(container.dispose);

    final options = RequestOptions(path: '/api/v1/items');
    final handler = _FakeRequestHandler();

    interceptor.onRequest(options, handler);

    expect(options.headers['Authorization'], 'Bearer test-token-123');
    expect(handler.nextCalled, isTrue);
  });

  test('no bearer header when controller has no token', () async {
    final (:interceptor, controller: _, :container) = _setup();
    addTearDown(container.dispose);

    final options = RequestOptions(path: '/api/v1/items');
    final handler = _FakeRequestHandler();

    interceptor.onRequest(options, handler);

    expect(options.headers['Authorization'], isNull);
    expect(handler.nextCalled, isTrue);
  });

  test('401 on auth-exempt path passes through without markExpired',
      () async {
    final (:interceptor, :controller, :container) =
        _setup(token: 'my-token');
    addTearDown(container.dispose);

    final err = DioException(
      requestOptions: RequestOptions(path: '/api/v1/auth/token'),
      response: Response(
        requestOptions: RequestOptions(path: '/api/v1/auth/token'),
        statusCode: 401,
      ),
    );
    final handler = _FakeErrorHandler();

    interceptor.onError(err, handler);
    // markExpired is unawaited; let microtasks drain.
    await Future<void>.delayed(Duration.zero);

    expect(controller.markExpiredCalls, 0);
    expect(handler.nextCalled, isTrue);
  });

  test('401 on normal path calls markExpired and passes the error on',
      () async {
    final (:interceptor, :controller, :container) =
        _setup(token: 'my-token');
    addTearDown(container.dispose);

    final err = DioException(
      requestOptions: RequestOptions(path: '/api/v1/items'),
      response: Response(
        requestOptions: RequestOptions(path: '/api/v1/items'),
        statusCode: 401,
      ),
    );
    final handler = _FakeErrorHandler();

    interceptor.onError(err, handler);
    await Future<void>.delayed(Duration.zero);

    expect(controller.markExpiredCalls, 1);
    expect(handler.nextCalled, isTrue);
  });

  test('non-401 error passes through without markExpired', () async {
    final (:interceptor, :controller, :container) =
        _setup(token: 'my-token');
    addTearDown(container.dispose);

    final err = DioException(
      requestOptions: RequestOptions(path: '/api/v1/items'),
      response: Response(
        requestOptions: RequestOptions(path: '/api/v1/items'),
        statusCode: 500,
      ),
    );
    final handler = _FakeErrorHandler();

    interceptor.onError(err, handler);
    await Future<void>.delayed(Duration.zero);

    expect(controller.markExpiredCalls, 0);
    expect(handler.nextCalled, isTrue);
  });
}

class _FakeRequestHandler extends RequestInterceptorHandler {
  bool nextCalled = false;

  @override
  void next(RequestOptions requestOptions) {
    nextCalled = true;
  }
}

class _FakeErrorHandler extends ErrorInterceptorHandler {
  bool nextCalled = false;

  @override
  void next(DioException err) {
    nextCalled = true;
  }
}
