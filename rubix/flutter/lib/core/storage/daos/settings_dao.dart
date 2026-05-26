import 'package:drift/drift.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';
import 'package:rubix_flutter/core/storage/tables/app_settings_table.dart';

part 'settings_dao.g.dart';

@DriftAccessor(tables: [AppSettings])
class SettingsDao extends DatabaseAccessor<AppDatabase>
    with _$SettingsDaoMixin {
  SettingsDao(super.db);

  Future<String?> getConnectionsPin() async {
    final row = await select(appSettings).getSingleOrNull();
    return row?.connectionsPin;
  }

  Future<void> setConnectionsPin(String? pin) async {
    await into(appSettings).insertOnConflictUpdate(
      AppSettingsCompanion.insert(
        connectionsPin: Value(pin),
      ),
    );
  }
}
