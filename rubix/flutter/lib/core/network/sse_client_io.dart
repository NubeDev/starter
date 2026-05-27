import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';

/// Native (VM / mobile / desktop) SSE implementation using Dio's
/// streaming response body. The `BrowserHttpClientAdapter` cannot
/// stream `ResponseType.stream` chunks, which is why a web-specific
/// `fetch()` impl lives in `sse_client_web.dart`.
Stream<String> openSseStream({
  required Dio dio,
  required String path,
  Map<String, Object?>? queryParameters,
  CancelToken? cancelToken,
}) async* {
  final response = await dio.get<ResponseBody>(
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
