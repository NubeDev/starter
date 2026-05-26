import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/network/auth_interceptor/auth_interceptor.dart';

/// In-memory token store for testing.
class _FakeTokenStore implements TokenStore {
  String? _token;
  int clearCount = 0;

  @override
  Future<String?> read() async => _token;

  @override
  Future<void> write(String token, {DateTime? expiresAt}) async {
    _token = token;
  }

  @override
  Future<void> clear() async {
    _token = null;
    clearCount++;
  }
}

/// Creates an AuthInterceptor wired to a test container with the fake store.
({AuthInterceptor interceptor, _FakeTokenStore store, ProviderContainer container}) _setup() {
  final fakeStore = _FakeTokenStore();
  final container = ProviderContainer(
    overrides: [
      tokenStoreProvider.overrideWithValue(fakeStore),
    ],
  );
  late AuthInterceptor interceptor;
  container.read(Provider((ref) {
    interceptor = AuthInterceptor(ref);
    return null;
  }));
  return (interceptor: interceptor, store: fakeStore, container: container);
}

void main() {
  test('bearer token is injected when token present', () async {
    final (:interceptor, :store, :container) = _setup();
    addTearDown(container.dispose);
    await store.write('test-token-123');

    final options = RequestOptions(path: '/api/v1/items');
    final handler = _FakeRequestHandler();

    await interceptor.onRequest(options, handler);

    expect(options.headers['Authorization'], 'Bearer test-token-123');
    expect(handler.nextCalled, isTrue);
  });

  test('no bearer header when no token', () async {
    final (:interceptor, store: _, :container) = _setup();
    addTearDown(container.dispose);

    final options = RequestOptions(path: '/api/v1/items');
    final handler = _FakeRequestHandler();

    await interceptor.onRequest(options, handler);

    expect(options.headers['Authorization'], isNull);
    expect(handler.nextCalled, isTrue);
  });

  test('401 on auth-exempt path does not evict token', () async {
    final (:interceptor, :store, :container) = _setup();
    addTearDown(container.dispose);
    await store.write('my-token');

    final err = DioException(
      requestOptions: RequestOptions(path: '/api/v1/auth/token'),
      response: Response(
        requestOptions: RequestOptions(path: '/api/v1/auth/token'),
        statusCode: 401,
      ),
    );
    final handler = _FakeErrorHandler();

    await interceptor.onError(err, handler);

    expect(store.clearCount, 0);
    expect(await store.read(), 'my-token');
  });

  test('401 on normal path evicts token exactly once (stampede)', () async {
    final (:interceptor, :store, :container) = _setup();
    addTearDown(container.dispose);
    await store.write('my-token');

    DioException makeErr() => DioException(
          requestOptions: RequestOptions(path: '/api/v1/items'),
          response: Response(
            requestOptions: RequestOptions(path: '/api/v1/items'),
            statusCode: 401,
          ),
        );

    // Fire two 401s concurrently (stampede scenario).
    await Future.wait([
      interceptor.onError(makeErr(), _FakeErrorHandler()),
      interceptor.onError(makeErr(), _FakeErrorHandler()),
    ]);

    // Token cleared exactly once thanks to the eviction lock.
    expect(store.clearCount, 1);
    expect(await store.read(), isNull);
  });

  test('non-401 error passes through without eviction', () async {
    final (:interceptor, :store, :container) = _setup();
    addTearDown(container.dispose);
    await store.write('my-token');

    final err = DioException(
      requestOptions: RequestOptions(path: '/api/v1/items'),
      response: Response(
        requestOptions: RequestOptions(path: '/api/v1/items'),
        statusCode: 500,
      ),
    );
    final handler = _FakeErrorHandler();

    await interceptor.onError(err, handler);

    expect(store.clearCount, 0);
    expect(await store.read(), 'my-token');
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
