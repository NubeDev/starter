import 'package:rubix_data/src/dto/connection_dto.dart';

/// Storage-agnostic interface for the connections table.
///
/// Two implementations exist:
/// - **Local** (native/desktop/mobile) — wraps the Drift `ConnectionDao`
///   directly and runs in-process.
/// - **REST** (web) — proxies every call to the companion Dart shelf
///   server, because browsers cannot open the on-disk SQLite file.
abstract class ConnectionsRepository {
  Future<List<ConnectionDto>> list();
  Future<int> add({required String label, required String baseUrl});
  Future<bool> update(int id, {String? label, String? baseUrl});
  Future<int> delete(int id);
  Future<void> setActive(int? connectionId);
  Future<int?> getActiveId();
  Future<bool> markUsed(int id);
}
