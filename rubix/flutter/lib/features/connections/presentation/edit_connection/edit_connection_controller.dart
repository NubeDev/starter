import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';

part 'edit_connection_controller.g.dart';

@riverpod
class EditConnectionController extends _$EditConnectionController {
  @override
  AsyncValue<void> build() => const AsyncData(null);

  Future<bool> update(int id, {required String label}) async {
    state = const AsyncLoading();
    final repo = ref.read(connectionRepositoryProvider);
    await repo.update(id, label: label);
    ref.invalidate(connectionListControllerProvider);
    state = const AsyncData(null);
    return true;
  }

  Future<bool> delete(int id) async {
    state = const AsyncLoading();
    final repo = ref.read(connectionRepositoryProvider);
    await repo.delete(id);
    ref.invalidate(connectionListControllerProvider);
    state = const AsyncData(null);
    return true;
  }
}
