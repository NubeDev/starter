import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/data/connection_repository.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

part 'add_connection_controller.g.dart';

@riverpod
class AddConnectionController extends _$AddConnectionController {
  @override
  AsyncValue<void> build() => const AsyncData(null);

  /// Probes the URL, persists the connection + credentials, then
  /// activates it (which triggers auto-login via [ConnectionListController]).
  Future<bool> submit({
    required String label,
    required String baseUrl,
    required String email,
    required String password,
  }) async {
    state = const AsyncLoading();
    final repo = ref.read(connectionRepositoryProvider);

    final result = await repo.probe(baseUrl);
    if (result is! ProbeOk) {
      final msg = switch (result) {
        ProbeTimeout() => 'Connection timed out',
        ProbeNon2xx(statusCode: final code) => 'Server returned $code',
        ProbeNetworkError(message: final m) => 'Network error: $m',
        _ => 'Probe failed',
      };
      state = AsyncError(msg, StackTrace.current);
      return false;
    }

    final id = await repo.add(label: label, baseUrl: baseUrl);
    await ref.read(connectionCredentialsStoreProvider).write(
          id,
          ConnectionCredentials(email: email, password: password),
        );
    ref.invalidate(connectionListControllerProvider);

    // Activate + auto-login. If login fails, surface the error but
    // keep the saved connection — the user can edit the password.
    try {
      await ref
          .read(connectionListControllerProvider.notifier)
          .activate(id);
    } catch (e) {
      state = AsyncError('Saved, but login failed: $e', StackTrace.current);
      return true;
    }

    state = const AsyncData(null);
    return true;
  }
}
