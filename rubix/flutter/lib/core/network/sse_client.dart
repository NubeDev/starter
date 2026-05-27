import 'package:dio/dio.dart';

// ignore: always_use_package_imports
import 'sse_client_io.dart'
    if (dart.library.js_interop) 'sse_client_web.dart' as impl;

/// Minimal Server-Sent Events client.
///
/// On native (VM / mobile / desktop) it streams via Dio's
/// `ResponseType.stream`. On web that path silently buffers because the
/// `BrowserHttpClientAdapter` (XHR) does not deliver chunks incrementally
/// for `ResponseType.stream` — so the web impl uses `fetch()` directly
/// with a `ReadableStream` reader, which DOES stream chunk-by-chunk
/// AND lets us keep the `Authorization` bearer header.
class SseClient {
  SseClient({required Dio dio}) : _dio = dio;

  final Dio _dio;

  /// Opens an SSE connection to [path] (resolved relative to the
  /// Dio `baseUrl`). The returned stream completes when the server
  /// closes the response; cancel by cancelling the subscription.
  Stream<String> connect(
    String path, {
    Map<String, Object?>? queryParameters,
    CancelToken? cancelToken,
  }) {
    return impl.openSseStream(
      dio: _dio,
      path: path,
      queryParameters: queryParameters,
      cancelToken: cancelToken,
    );
  }
}
