import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';

import 'package:dio/dio.dart';
import 'package:web/web.dart' as web;

/// Web SSE implementation using `fetch()` + `ReadableStream`.
///
/// Why not `package:dio` here: the browser `XMLHttpRequest` adapter
/// dio falls back to does not deliver `ResponseType.stream` chunks
/// incrementally — the whole response is buffered until the server
/// closes the connection, which for SSE is never. `fetch()` exposes a
/// `ReadableStream` body that DOES stream chunk-by-chunk and still
/// lets us set the `Authorization` header (unlike `EventSource`).
Stream<String> openSseStream({
  required Dio dio,
  required String path,
  Map<String, Object?>? queryParameters,
  CancelToken? cancelToken,
}) {
  final controller = StreamController<String>();

  Future<void> run() async {
    web.AbortController? abort;
    try {
      final baseUrl = dio.options.baseUrl;
      final qp = _queryString(queryParameters);
      final joined = _joinUrl(baseUrl, path);
      final url = qp.isEmpty ? joined : '$joined?$qp';

      // Run dio's request interceptors against a stub so the
      // AuthInterceptor (and any future header-mutating interceptor)
      // populates the same headers it would for a normal GET —
      // including the dynamic `Authorization: Bearer …`.
      final stubOptions = RequestOptions(
        path: path,
        method: 'GET',
        baseUrl: baseUrl,
        queryParameters: queryParameters,
        headers: <String, dynamic>{
          ...dio.options.headers,
          'Accept': 'text/event-stream',
        },
      );
      await _runRequestInterceptors(dio, stubOptions);

      final headersJs = web.Headers();
      stubOptions.headers.forEach((k, v) {
        if (v != null) headersJs.set(k, v.toString());
      });

      abort = web.AbortController();
      unawaited(cancelToken?.whenCancel.then((_) => abort?.abort()));

      // `credentials: 'omit'` — bearer auth is in the header, and the
      // server's CORS is `Allow-Origin: *` (very_permissive), which
      // browsers reject combined with `include`. With `omit` the
      // Authorization header still flows through unaffected.
      final init = web.RequestInit(
        method: 'GET',
        headers: headersJs,
        signal: abort.signal,
        credentials: 'omit',
      );

      // ignore: avoid_print
      print('SSE fetch → $url  headers=${stubOptions.headers}');
      final response = await web.window.fetch(url.toJS, init).toDart;
      if (!response.ok) {
        controller.addError(
          DioException(
            requestOptions: stubOptions,
            response: Response<dynamic>(
              requestOptions: stubOptions,
              statusCode: response.status,
            ),
            message: 'SSE connect failed: HTTP ${response.status}',
          ),
        );
        await controller.close();
        return;
      }

      final body = response.body;
      if (body == null) {
        await controller.close();
        return;
      }

      // `getReader()` returns the opaque `ReadableStreamReader` typedef;
      // wrap as the default reader so `read()` is available.
      final reader = web.ReadableStreamDefaultReader(body);
      final buffer = StringBuffer();
      final dataLines = <String>[];

      while (true) {
        final chunk = await reader.read().toDart;
        if (chunk.done) break;
        final value = chunk.value;
        if (value == null) continue;
        final bytes = (value as JSUint8Array).toDart;
        buffer.write(utf8.decode(bytes, allowMalformed: true));

        while (true) {
          final raw = buffer.toString();
          final nlIndex = raw.indexOf('\n');
          if (nlIndex < 0) break;

          final line = raw.substring(0, nlIndex).replaceAll('\r', '');
          buffer
            ..clear()
            ..write(raw.substring(nlIndex + 1));

          if (line.isEmpty) {
            if (dataLines.isNotEmpty) {
              controller.add(dataLines.join('\n'));
              dataLines.clear();
            }
            continue;
          }
          if (line.startsWith(':')) continue;
          if (line.startsWith('data:')) {
            dataLines.add(
              line.substring(5).startsWith(' ')
                  ? line.substring(6)
                  : line.substring(5),
            );
          }
        }
      }
      await controller.close();
    } catch (e, st) {
      if (!controller.isClosed) {
        controller.addError(e, st);
        await controller.close();
      }
    } finally {
      abort?.abort();
    }
  }

  controller.onListen = run;
  return controller.stream;
}

String _joinUrl(String base, String path) {
  if (path.startsWith('http://') || path.startsWith('https://')) return path;
  final b = base.endsWith('/') ? base.substring(0, base.length - 1) : base;
  final p = path.startsWith('/') ? path : '/$path';
  return '$b$p';
}

String _queryString(Map<String, Object?>? params) {
  if (params == null || params.isEmpty) return '';
  final pairs = <String>[];
  params.forEach((k, v) {
    if (v == null) return;
    pairs.add('${Uri.encodeQueryComponent(k)}='
        '${Uri.encodeQueryComponent(v.toString())}');
  });
  return pairs.join('&');
}

/// Walk dio's registered interceptors and let each `onRequest` mutate
/// [options]. Mirrors what `dio.fetch` does internally so the
/// AuthInterceptor's bearer header lands on the SSE request too.
Future<void> _runRequestInterceptors(Dio dio, RequestOptions options) async {
  for (final interceptor in dio.interceptors) {
    final completer = Completer<void>();
    final handler = _CapturingRequestHandler(completer, options);
    interceptor.onRequest(options, handler);
    await completer.future;
    if (handler.aborted) return;
  }
}

class _CapturingRequestHandler extends RequestInterceptorHandler {
  _CapturingRequestHandler(this._completer, this._options);

  final Completer<void> _completer;
  final RequestOptions _options;
  bool aborted = false;

  @override
  void next(RequestOptions options) {
    // Interceptors typically mutate `options.headers` in place and
    // pass the SAME reference to `handler.next(options)` — so
    // _options.headers IS options.headers and any "copy" would be a
    // no-op (or worse, a clear+self-assign that wipes the additions).
    // Only copy when the interceptor handed us a different instance.
    if (!identical(options, _options)) {
      _options.headers
        ..clear()
        ..addAll(options.headers);
    }
    if (!_completer.isCompleted) _completer.complete();
  }

  @override
  void resolve(
    Response<dynamic> response, [
    bool callFollowingResponseInterceptor = false,
  ]) {
    aborted = true;
    if (!_completer.isCompleted) _completer.complete();
  }

  @override
  void reject(
    DioException error, [
    bool callFollowingErrorInterceptor = false,
  ]) {
    aborted = true;
    if (!_completer.isCompleted) _completer.completeError(error);
  }
}
