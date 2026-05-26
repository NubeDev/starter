import 'package:dio/dio.dart';
import 'package:rubix_flutter/core/network/dio_client.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';
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

class ConnectionRepository {
  ConnectionRepository(this._dao);

  final ConnectionDao _dao;

  Future<List<Connection>> list() async {
    final rows = await _dao.list();
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

  Future<int> add({required String label, required String baseUrl}) =>
      _dao.insert(label: label, baseUrl: baseUrl);

  Future<bool> update(int id, {String? label, String? baseUrl}) =>
      _dao.updateConnection(id, label: label, baseUrl: baseUrl);

  Future<int> delete(int id) => _dao.deleteConnection(id);

  Future<void> setActive(int? connectionId) => _dao.setActive(connectionId);

  Future<int?> getActiveId() => _dao.getActiveId();

  Future<bool> markUsed(int id) => _dao.markUsed(id);

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
