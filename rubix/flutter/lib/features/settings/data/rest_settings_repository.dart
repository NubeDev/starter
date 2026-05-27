import 'package:dio/dio.dart';
import 'package:rubix_data/rubix_data.dart';

/// Web-mode impl of [SettingsRepository] — hits the companion
/// `rubix_server` shelf process.
class RestSettingsRepository implements SettingsRepository {
  RestSettingsRepository({required String baseUrl, Dio? dio})
      : _dio = (dio ?? Dio())..options.baseUrl = baseUrl;

  final Dio _dio;

  @override
  Future<String?> getConnectionsPin() async {
    final res = await _dio.get<Map<String, dynamic>>(
      '/api/settings/connections-pin',
    );
    return res.data!['pin'] as String?;
  }

  @override
  Future<void> setConnectionsPin(String? pin) async {
    await _dio.put<void>(
      '/api/settings/connections-pin',
      data: {'pin': pin},
    );
  }
}
