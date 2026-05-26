import 'package:drift/drift.dart';

/// Single-row table holding app-wide preferences that benefit from
/// SQL persistence (as opposed to `shared_preferences`-style scalars
/// like theme / locale, see APP-SHELL.md → "Why two storage layers").
///
/// Currently just the optional PIN that gates `/connections*`. The
/// PIN is stored as plaintext — this is a local lock for casual
/// shoulder-surfing protection, not an authentication credential.
/// Anyone with file-system access to the SQLite DB already has the
/// stored auth token, so hashing the PIN here would not raise the
/// security floor.
@DataClassName('AppSettingsEntry')
class AppSettings extends Table {
  IntColumn get id => integer().withDefault(const Constant(1))();
  TextColumn get connectionsPin => text().nullable()();

  @override
  Set<Column> get primaryKey => {id};
}
