import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:pretty_dio_logger/pretty_dio_logger.dart';
import 'package:rubix_flutter/core/network/auth_interceptor/auth_interceptor.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/core/demo/demo_mode.dart';
import 'package:rubix_flutter/core/demo/demo_repositories.dart' as demo;
// Example: swap repository providers for demo mode
final userRepositoryProvider = Provider((ref) {
  if (demoMode) return demo.DemoUserRepository();
  // TODO: Replace with real UserRepository
  throw UnimplementedError('Real UserRepository not wired');
});

final deviceRepositoryProvider = Provider((ref) {
  if (demoMode) return demo.DemoDeviceRepository();
  // TODO: Replace with real DeviceRepository
  throw UnimplementedError('Real DeviceRepository not wired');
});

final agentRepositoryProvider = Provider((ref) {
  // TODO: Replace with real AgentRepository
  throw UnimplementedError('Real AgentRepository not wired');
});

final metricRepositoryProvider = Provider((ref) {
  if (demoMode) return demo.DemoMetricRepository();
  // TODO: Replace with real MetricRepository
  throw UnimplementedError('Real MetricRepository not wired');
});

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
