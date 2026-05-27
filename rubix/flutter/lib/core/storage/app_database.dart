import 'package:drift/drift.dart';
import 'package:drift_flutter/drift_flutter.dart';
import 'package:flutter/foundation.dart';

import 'package:rubix_flutter/core/storage/tables/app_settings_table.dart';
import 'package:rubix_flutter/core/storage/tables/connection_state_table.dart';
import 'package:rubix_flutter/core/storage/tables/connections_table.dart';

part 'app_database.g.dart';

@DriftDatabase(tables: [Connections, ConnectionState, AppSettings])
class AppDatabase extends _$AppDatabase {
  AppDatabase() : super(_openConnection());

  AppDatabase.forTesting(super.e);

  @override
  int get schemaVersion => 2;

  @override
  MigrationStrategy get migration => MigrationStrategy(
        onCreate: (m) => m.createAll(),
        onUpgrade: (m, from, to) async {
          if (from < 2) {
            await m.createTable(appSettings);
          }
        },
      );

  static QueryExecutor _openConnection() {
    debugPrint('[DB] _openConnection() called');
    return driftDatabase(
      name: 'rubix_app',
      web: DriftWebOptions(
        sqlite3Wasm: Uri.parse('sqlite3.wasm'),
        driftWorker: Uri.parse('drift_worker.dart.js'),
        onResult: (result) {
          debugPrint(
            '[DB] drift web chose=${result.chosenImplementation} '
            'missing=${result.missingFeatures}',
          );
        },
      ),
    );
  }
}
