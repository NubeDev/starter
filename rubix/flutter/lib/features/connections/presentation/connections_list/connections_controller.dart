import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/core/auth/token_store/token_store_providers.dart';
import 'package:rubix_flutter/core/storage/data_layer.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/data/connection_repository.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';

part 'connections_controller.g.dart';

@riverpod
ConnectionRepository connectionRepository(Ref ref) {
  return ConnectionRepository(ref.watch(connectionsRepositoryProvider));
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
  ///
  /// The token-store clear + auth-state reset are deliberate side
  /// effects of *connection* activation, not of the auth flow itself:
  /// the new active connection means the previous server's token is
  /// not valid here. `AuthController.login(...)` then runs a fresh
  /// issue against the now-active baseUrl.
  Future<void> activate(int id) async {
    await ref.read(connectionRepositoryProvider).setActive(id);
    ref.invalidate(activeConnectionProvider);
    await refresh();

    await ref.read(tokenStoreProvider).clear();

    final creds = await ref.read(connectionCredentialsStoreProvider).read(id);
    if (creds == null) {
      throw StateError('No saved credentials for this connection');
    }

    await ref.read(authControllerProvider.notifier).login(
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
