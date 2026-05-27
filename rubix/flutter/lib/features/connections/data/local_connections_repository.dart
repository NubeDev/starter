import 'package:rubix_data/rubix_data.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';

/// Native/desktop/mobile implementation of [ConnectionsRepository].
///
/// Thin wrapper over the existing Drift [ConnectionDao] — the only job
/// is to translate Drift's generated `Connection` rows into the wire
/// DTO that the REST path also produces, so callers above the
/// repository are storage-agnostic.
class LocalConnectionsRepository implements ConnectionsRepository {
  LocalConnectionsRepository(this._dao);

  final ConnectionDao _dao;

  @override
  Future<List<ConnectionDto>> list() async {
    final rows = await _dao.list();
    return rows
        .map((r) => ConnectionDto(
              id: r.id,
              label: r.label,
              baseUrl: r.baseUrl,
              createdAt: r.createdAt,
              lastUsedAt: r.lastUsedAt,
            ))
        .toList();
  }

  @override
  Future<int> add({required String label, required String baseUrl}) =>
      _dao.insert(label: label, baseUrl: baseUrl);

  @override
  Future<bool> update(int id, {String? label, String? baseUrl}) =>
      _dao.updateConnection(id, label: label, baseUrl: baseUrl);

  @override
  Future<int> delete(int id) => _dao.deleteConnection(id);

  @override
  Future<void> setActive(int? connectionId) => _dao.setActive(connectionId);

  @override
  Future<int?> getActiveId() => _dao.getActiveId();

  @override
  Future<bool> markUsed(int id) => _dao.markUsed(id);
}
