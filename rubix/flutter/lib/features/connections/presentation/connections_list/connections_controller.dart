import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/app.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/storage/daos/connection_dao.dart';
import 'package:rubix_flutter/features/auth/data/auth_repository/auth_repository.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
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
    await ref.read(connectionCredentialsStoreProvider).delete(id);
    await refresh();
  }

  /// Switch to a connection and auto-issue a token using its stored
  /// credentials. Throws if no creds are saved or login fails.
  Future<void> activate(int id) async {
    await ref.read(connectionRepositoryProvider).setActive(id);
    ref.invalidate(activeConnectionProvider);
    await refresh();

    // Fresh activation — clear the circuit breaker so currentTokenProvider
    // will auto-relogin on the new connection.
    ref.read(authGaveUpProvider.notifier).set(false);

    // Clear any existing token from the previous connection.
    await ref.read(tokenStoreProvider).clear();

    final creds = await ref.read(connectionCredentialsStoreProvider).read(id);
    if (creds == null) {
      ref.invalidate(currentTokenProvider);
      throw StateError('No saved credentials for this connection');
    }

    await ref.read(authRepositoryProvider).login(
          email: creds.email,
          password: creds.password,
        );
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
