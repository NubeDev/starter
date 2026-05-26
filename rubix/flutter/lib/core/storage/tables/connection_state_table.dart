import 'package:drift/drift.dart';

/// Single-row table tracking which connection is currently active.
class ConnectionState extends Table {
  IntColumn get id =>
      integer().withDefault(const Constant(1)).unique()();
  IntColumn get activeConnectionId => integer().nullable()();
}
