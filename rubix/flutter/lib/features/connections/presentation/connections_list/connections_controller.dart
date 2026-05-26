import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/app.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';
import 'package:rubix_flutter/features/connections/data/connection_repository.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';

part 'connections_controller.g.dart';

@riverpod
ConnectionRepository connectionRepository(Ref ref) {
  final db = ref.watch(appDatabaseProvider);
  return ConnectionRepository(ConnectionDao(db));
}

@riverpod
class ConnectionListController extends _$ConnectionListController {
  @override
  Future<List<Connection>> build() async {
    final repo = ref.watch(connectionRepositoryProvider);
    return repo.list();
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(
      () => ref.read(connectionRepositoryProvider).list(),
    );
  }

  Future<void> add({required String label, required String baseUrl}) async {
    await ref.read(connectionRepositoryProvider).add(
      label: label,
      baseUrl: baseUrl,
    );
    await refresh();
  }

  Future<void> delete(int id) async {
    await ref.read(connectionRepositoryProvider).delete(id);
    await refresh();
  }

  Future<void> activate(int id) async {
    await ref.read(connectionRepositoryProvider).setActive(id);
    ref.invalidate(activeConnectionProvider);
    await refresh();
  }
}

@riverpod
Future<Connection?> activeConnection(Ref ref) async {
  final repo = ref.watch(connectionRepositoryProvider);
  final activeId = await repo.getActiveId();
  if (activeId == null) return null;
  final all = await repo.list();
  return all.where((c) => c.id == activeId).firstOrNull;
}
