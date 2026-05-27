import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';

/// Minimal Server-Sent Events client over Dio.
///
/// Streams `data:` payloads from a long-lived GET response and
/// emits one event per blank-line-terminated SSE frame. Comment
/// lines (`: keepalive`) and `event:` / `id:` / `retry:` fields are
/// ignored — callers only need the `data:` body for the dashboard
/// / flow event streams in this app.
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
  }) async* {
    final response = await _dio.get<ResponseBody>(
      path,
      queryParameters: queryParameters,
      cancelToken: cancelToken,
      options: Options(
        responseType: ResponseType.stream,
        headers: {'Accept': 'text/event-stream'},
      ),
    );

    final body = response.data;
    if (body == null) return;

    final buffer = StringBuffer();
    final dataLines = <String>[];

    await for (final chunk in body.stream) {
      buffer.write(utf8.decode(chunk, allowMalformed: true));

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
            yield dataLines.join('\n');
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
        // event:/id:/retry: lines are intentionally ignored.
      }
    }
  }
}
