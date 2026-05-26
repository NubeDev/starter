import 'package:drift/native.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';

void main() {
  late AppDatabase db;

  setUp(() {
    db = AppDatabase.forTesting(NativeDatabase.memory());
  });

  tearDown(() async {
    await db.close();
  });

  test('insert and read a connection', () async {
    await db.into(db.connections).insert(
      ConnectionsCompanion.insert(
        label: 'Local Agent',
        baseUrl: 'http://localhost:8088',
      ),
    );

    final rows = await db.select(db.connections).get();
    expect(rows, hasLength(1));
    expect(rows.first.label, 'Local Agent');
    expect(rows.first.baseUrl, 'http://localhost:8088');
  });
}
