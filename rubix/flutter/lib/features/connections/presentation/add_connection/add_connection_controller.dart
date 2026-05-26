import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/features/connections/data/connection_repository.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

part 'add_connection_controller.g.dart';

@riverpod
class AddConnectionController extends _$AddConnectionController {
  @override
  AsyncValue<void> build() => const AsyncData(null);

  Future<bool> submit({
    required String label,
    required String baseUrl,
  }) async {
    state = const AsyncLoading();
    final repo = ref.read(connectionRepositoryProvider);

    // Probe first
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

    // Save
    await repo.add(label: label, baseUrl: baseUrl);
    ref.invalidate(connectionListControllerProvider);
    state = const AsyncData(null);
    return true;
  }
}
