import 'package:drift/drift.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';
import 'package:rubix_flutter/core/storage/tables/connection_state_table.dart';
import 'package:rubix_flutter/core/storage/tables/connections_table.dart';

part 'connection_dao.g.dart';

@DriftAccessor(tables: [Connections, ConnectionState])
class ConnectionDao extends DatabaseAccessor<AppDatabase>
    with _$ConnectionDaoMixin {
  ConnectionDao(super.db);

  Future<List<Connection>> list() => select(connections).get();

  Future<int> insert({required String label, required String baseUrl}) =>
      into(connections).insert(
        ConnectionsCompanion.insert(label: label, baseUrl: baseUrl),
      );

  Future<bool> updateConnection(int id, {String? label, String? baseUrl}) =>
      (update(connections)..where((t) => t.id.equals(id))).write(
        ConnectionsCompanion(
          label: label != null ? Value(label) : const Value.absent(),
          baseUrl: baseUrl != null ? Value(baseUrl) : const Value.absent(),
        ),
      ).then((rows) => rows > 0);

  Future<int> deleteConnection(int id) =>
      (delete(connections)..where((t) => t.id.equals(id))).go();

  Future<void> setActive(int? connectionId) async {
    await into(connectionState).insertOnConflictUpdate(
      ConnectionStateCompanion.insert(
        activeConnectionId: Value(connectionId),
      ),
    );
  }

  Future<int?> getActiveId() async {
    final row = await select(connectionState).getSingleOrNull();
    return row?.activeConnectionId;
  }

  Future<bool> markUsed(int id) =>
      (update(connections)..where((t) => t.id.equals(id))).write(
        ConnectionsCompanion(lastUsedAt: Value(DateTime.now())),
      ).then((rows) => rows > 0);
}
