import 'package:drift/drift.dart';
import 'package:flutter/foundation.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';
import 'package:rubix_flutter/core/storage/tables/connection_state_table.dart';
import 'package:rubix_flutter/core/storage/tables/connections_table.dart';

part 'connection_dao.g.dart';

@DriftAccessor(tables: [Connections, ConnectionState])
class ConnectionDao extends DatabaseAccessor<AppDatabase>
    with _$ConnectionDaoMixin {
  ConnectionDao(super.db);

  Future<List<Connection>> list() async {
    final rows = await select(connections).get();
    debugPrint('[DB] ConnectionDao.list() → ${rows.length} rows');
    for (final r in rows) {
      debugPrint('[DB]   row id=${r.id} label=${r.label} baseUrl=${r.baseUrl}');
    }
    return rows;
  }

  Future<int> insert({required String label, required String baseUrl}) async {
    debugPrint('[DB] ConnectionDao.insert(label=$label, baseUrl=$baseUrl)');
    final id = await into(connections).insert(
      ConnectionsCompanion.insert(label: label, baseUrl: baseUrl),
    );
    debugPrint('[DB] ConnectionDao.insert → id=$id');
    final verify = await select(connections).get();
    debugPrint('[DB]   post-insert verify: ${verify.length} rows in table');
    return id;
  }

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
    // Singleton row pinned at id=1. Pass id explicitly so the upsert has
    // a deterministic PK to conflict on (don't rely on SQL DEFAULT 1 — Drift
    // generates `INSERT INTO connection_state DEFAULT VALUES` when both
    // columns are absent, which inserts a single row but then ON CONFLICT
    // on subsequent calls only fires if id actually matches).
    await into(connectionState).insertOnConflictUpdate(
      ConnectionStateCompanion.insert(
        id: const Value(1),
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
