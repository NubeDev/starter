import 'package:dio/dio.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/core/network/dio_client.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/domain/me_response/me_response.dart';

part 'home_controller.g.dart';

/// Status of the rubix-agent `/healthz` probe shown on the home screen.
sealed class AgentHealth {
  const AgentHealth();
}

/// Agent responded 2xx to `/healthz`.
class AgentHealthOk extends AgentHealth {
  const AgentHealthOk();
}

/// Agent could not be reached (network error or timeout).
class AgentHealthUnreachable extends AgentHealth {
  const AgentHealthUnreachable(this.message);
  final String message;
}

/// Agent responded but with a non-2xx status.
class AgentHealthBadStatus extends AgentHealth {
  const AgentHealthBadStatus(this.statusCode);
  final int statusCode;
}

/// Calls `GET /healthz` on the active connection.
///
/// Uses a bare [probeDio] (no bearer required) and joins the path
/// against the active connection's `baseUrl`.
@riverpod
Future<AgentHealth> agentHealth(Ref ref) async {
  final active = await ref.watch(activeConnectionProvider.future);
  if (active == null) {
    return const AgentHealthUnreachable('No active connection');
  }

  final dio = probeDio();
  final url = active.baseUrl.endsWith('/')
      ? '${active.baseUrl}healthz'
      : '${active.baseUrl}/healthz';

  try {
    final response = await dio.get<dynamic>(url);
    final code = response.statusCode ?? 0;
    if (code >= 200 && code < 300) {
      return const AgentHealthOk();
    }
    return AgentHealthBadStatus(code);
  } on DioException catch (e) {
    return AgentHealthUnreachable(e.message ?? 'Network error');
  } finally {
    dio.close();
  }
}

/// Calls `GET /api/v1/auth/me` on the active connection with bearer auth.
@riverpod
Future<MeResponse> currentUser(Ref ref) async {
  final dio = ref.watch(dioProvider);
  if (dio == null) {
    throw StateError('No active connection');
  }
  final response = await dio.get<Map<String, dynamic>>('/api/v1/auth/me');
  return MeResponse.fromJson(response.data!);
}
