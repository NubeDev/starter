import 'package:drift/drift.dart';

/// Stores saved rubix-agent connections.
class Connections extends Table {
  IntColumn get id => integer().autoIncrement()();
  TextColumn get label => text().withLength(min: 1, max: 128)();
  TextColumn get baseUrl => text()();
  DateTimeColumn get createdAt => dateTime().withDefault(currentDateAndTime)();
  DateTimeColumn get lastUsedAt => dateTime().nullable()();
}
