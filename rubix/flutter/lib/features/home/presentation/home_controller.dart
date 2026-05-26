import 'package:dio/dio.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/core/api/api_providers.dart';
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

/// Calls `GET /healthz` on the active connection via the generated
/// `rubix_api` Dio client (see `apiClientProvider`). The shared Dio
/// instance carries the AuthInterceptor, but `/healthz` is on the
/// exempt list so no bearer is required.
@riverpod
Future<AgentHealth> agentHealth(Ref ref) async {
  final api = ref.watch(apiClientProvider);
  if (api == null) {
    return const AgentHealthUnreachable('No active connection');
  }

  try {
    final response = await api.getSystemApi().healthz();
    final code = response.statusCode ?? 0;
    if (code >= 200 && code < 300) {
      return const AgentHealthOk();
    }
    return AgentHealthBadStatus(code);
  } on DioException catch (e) {
    final code = e.response?.statusCode;
    if (code != null && (code < 200 || code >= 300)) {
      return AgentHealthBadStatus(code);
    }
    return AgentHealthUnreachable(e.message ?? 'Network error');
  }
}

/// Calls `GET /api/v1/auth/me` on the active connection with bearer auth
/// via the generated `rubix_api` Dio client.
@riverpod
Future<MeResponse> currentUser(Ref ref) async {
  final api = ref.watch(apiClientProvider);
  if (api == null) {
    throw StateError('No active connection');
  }
  final response = await api.getAuthApi().me();
  final data = response.data;
  if (data == null) {
    throw StateError('Empty /auth/me response');
  }
  return MeResponse(
    subject: data.subject,
    email: data.email,
    role: data.role.name,
  );
}
