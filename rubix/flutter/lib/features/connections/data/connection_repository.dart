import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:rubix_data/rubix_data.dart';
import 'package:rubix_flutter/core/network/dio_client.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';

/// Result of probing a rubix-agent healthz endpoint.
sealed class ProbeResult {
  const ProbeResult();
}

class ProbeOk extends ProbeResult {
  const ProbeOk();
}

class ProbeTimeout extends ProbeResult {
  const ProbeTimeout();
}

class ProbeNon2xx extends ProbeResult {
  const ProbeNon2xx(this.statusCode);
  final int statusCode;
}

class ProbeNetworkError extends ProbeResult {
  const ProbeNetworkError(this.message);
  final String message;
}

/// Feature-level facade over [ConnectionsRepository].
///
/// The data-layer interface (`rubix_data`) returns plain DTOs and
/// knows nothing about Freezed or rubix-agent health probes — those
/// belong here. The concrete store (Drift on native, REST on web) is
/// injected, so the same UI/controller code runs against both.
class ConnectionRepository {
  ConnectionRepository(this._store);

  final ConnectionsRepository _store;

  Future<List<Connection>> list() async {
    debugPrint('[REPO] list() called');
    final rows = await _store.list();
    debugPrint('[REPO] list() returning ${rows.length} connection(s)');
    return rows
        .map(
          (r) => Connection(
            id: r.id,
            label: r.label,
            baseUrl: r.baseUrl,
            createdAt: r.createdAt,
            lastUsedAt: r.lastUsedAt,
          ),
        )
        .toList();
  }

  Future<int> add({required String label, required String baseUrl}) {
    debugPrint('[REPO] add(label=$label, baseUrl=$baseUrl)');
    return _store.add(label: label, baseUrl: baseUrl);
  }

  Future<bool> update(int id, {String? label, String? baseUrl}) =>
      _store.update(id, label: label, baseUrl: baseUrl);

  Future<int> delete(int id) => _store.delete(id);

  Future<void> setActive(int? connectionId) => _store.setActive(connectionId);

  Future<int?> getActiveId() => _store.getActiveId();

  Future<bool> markUsed(int id) => _store.markUsed(id);

  Future<ProbeResult> probe(String baseUrl) async {
    try {
      final dio = probeDio();
      final uri = baseUrl.endsWith('/')
          ? '${baseUrl}healthz'
          : '$baseUrl/healthz';
      final response = await dio.get<dynamic>(uri);
      if (response.statusCode != null &&
          response.statusCode! >= 200 &&
          response.statusCode! < 300) {
        return const ProbeOk();
      }
      return ProbeNon2xx(response.statusCode ?? 0);
    } on DioException catch (e) {
      if (e.type == DioExceptionType.connectionTimeout ||
          e.type == DioExceptionType.receiveTimeout) {
        return const ProbeTimeout();
      }
      if (e.response != null) {
        return ProbeNon2xx(e.response!.statusCode ?? 0);
      }
      return ProbeNetworkError(e.message ?? 'Unknown network error');
    }
  }
}
