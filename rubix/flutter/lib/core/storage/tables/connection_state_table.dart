import 'package:drift/drift.dart';

/// Single-row table tracking which connection is currently active.
@DataClassName('ConnectionStateEntry')
class ConnectionState extends Table {
  IntColumn get id => integer().withDefault(const Constant(1))();
  IntColumn get activeConnectionId => integer().nullable()();

  @override
  Set<Column> get primaryKey => {id};
}
