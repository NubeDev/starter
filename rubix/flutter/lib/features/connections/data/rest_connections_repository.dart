import 'package:dio/dio.dart';
import 'package:rubix_data/rubix_data.dart';

/// Web-mode implementation of [ConnectionsRepository] — talks to the
/// companion Dart shelf server at [baseUrl] (default
/// `http://localhost:8787`). Used because browsers can't open the
/// SQLite file that the native build reads directly.
class RestConnectionsRepository implements ConnectionsRepository {
  RestConnectionsRepository({required String baseUrl, Dio? dio})
      : _dio = (dio ?? Dio())..options.baseUrl = baseUrl;

  final Dio _dio;

  @override
  Future<List<ConnectionDto>> list() async {
    final res = await _dio.get<List<dynamic>>('/api/connections');
    return (res.data ?? const [])
        .cast<Map<String, dynamic>>()
        .map(ConnectionDto.fromJson)
        .toList();
  }

  @override
  Future<int> add({required String label, required String baseUrl}) async {
    final res = await _dio.post<Map<String, dynamic>>(
      '/api/connections',
      data: {'label': label, 'baseUrl': baseUrl},
    );
    return res.data!['id'] as int;
  }

  @override
  Future<bool> update(int id, {String? label, String? baseUrl}) async {
    final res = await _dio.patch<Map<String, dynamic>>(
      '/api/connections/$id',
      data: {
        if (label != null) 'label': label,
        if (baseUrl != null) 'baseUrl': baseUrl,
      },
    );
    return res.data!['updated'] as bool;
  }

  @override
  Future<int> delete(int id) async {
    final res = await _dio.delete<Map<String, dynamic>>('/api/connections/$id');
    return (res.data!['deleted'] as int?) ?? 0;
  }

  @override
  Future<void> setActive(int? connectionId) async {
    await _dio.put<void>(
      '/api/connections/active',
      data: {'activeId': connectionId},
    );
  }

  @override
  Future<int?> getActiveId() async {
    final res = await _dio.get<Map<String, dynamic>>('/api/connections/active');
    return res.data!['activeId'] as int?;
  }

  @override
  Future<bool> markUsed(int id) async {
    final res = await _dio.post<Map<String, dynamic>>(
      '/api/connections/$id/mark-used',
    );
    return res.data!['updated'] as bool;
  }
}
