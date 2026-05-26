import 'package:drift/native.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_flutter/core/storage/app_database.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';
import 'package:rubix_flutter/features/connections/data/connection_repository.dart';

void main() {
  late AppDatabase db;
  late ConnectionRepository repo;

  setUp(() {
    db = AppDatabase.forTesting(NativeDatabase.memory());
    repo = ConnectionRepository(ConnectionDao(db));
  });

  tearDown(() async {
    await db.close();
  });

  test('add and list connections', () async {
    expect(await repo.list(), isEmpty);

    await repo.add(label: 'Agent 1', baseUrl: 'http://localhost:8088');
    await repo.add(label: 'Agent 2', baseUrl: 'http://192.168.1.5:8088');

    final connections = await repo.list();
    expect(connections, hasLength(2));
    expect(connections[0].label, 'Agent 1');
    expect(connections[1].label, 'Agent 2');
  });

  test('update connection label', () async {
    await repo.add(label: 'Old', baseUrl: 'http://x:8088');
    final before = await repo.list();
    await repo.update(before.first.id, label: 'New');
    final after = await repo.list();
    expect(after.first.label, 'New');
  });

  test('delete connection', () async {
    await repo.add(label: 'Tmp', baseUrl: 'http://x:8088');
    final connections = await repo.list();
    await repo.delete(connections.first.id);
    expect(await repo.list(), isEmpty);
  });

  test('set and get active connection', () async {
    await repo.add(label: 'A', baseUrl: 'http://a:8088');
    final connections = await repo.list();
    final id = connections.first.id;

    expect(await repo.getActiveId(), isNull);
    await repo.setActive(id);
    expect(await repo.getActiveId(), id);
  });

  test('markUsed updates lastUsedAt', () async {
    await repo.add(label: 'M', baseUrl: 'http://m:8088');
    final connections = await repo.list();
    expect(connections.first.lastUsedAt, isNull);

    await repo.markUsed(connections.first.id);
    final updated = await repo.list();
    expect(updated.first.lastUsedAt, isNotNull);
  });
}
