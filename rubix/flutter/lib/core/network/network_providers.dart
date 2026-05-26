import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:pretty_dio_logger/pretty_dio_logger.dart';
import 'package:rubix_flutter/core/network/auth_interceptor/auth_interceptor.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

/// Dio instance wired to the active connection's baseUrl.
/// Rebuilt only when baseUrl changes — token changes do NOT invalidate.
final dioProvider = Provider<Dio?>((ref) {
  final active = ref.watch(
    activeConnectionProvider.select((a) => a.value?.baseUrl),
  );
  if (active == null) return null;

  final dio = Dio(
    BaseOptions(
      baseUrl: active,
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 30),
    ),
  );

  dio.interceptors.add(AuthInterceptor(ref));

  if (kDebugMode) {
    dio.interceptors.add(
      PrettyDioLogger(requestBody: true),
    );
  }

  return dio;
});
