import 'package:drift/drift.dart';
import 'package:drift_flutter/drift_flutter.dart';

import 'package:rubix_flutter/core/storage/tables/connection_state_table.dart';
import 'package:rubix_flutter/core/storage/tables/connections_table.dart';

part 'app_database.g.dart';

@DriftDatabase(tables: [Connections, ConnectionState])
class AppDatabase extends _$AppDatabase {
  AppDatabase() : super(_openConnection());

  AppDatabase.forTesting(super.e);

  @override
  int get schemaVersion => 1;

  static QueryExecutor _openConnection() {
    return driftDatabase(
      name: 'rubix_app',
      web: DriftWebOptions(
        sqlite3Wasm: Uri.parse('sqlite3.wasm'),
        driftWorker: Uri.parse('drift_worker.dart.js'),
      ),
    );
  }
}
