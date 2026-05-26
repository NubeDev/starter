import 'package:dio/dio.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:rubix_flutter/features/auth/data/auth_repository/auth_repository.dart';

part 'login_controller.g.dart';

@riverpod
class LoginController extends _$LoginController {
  @override
  AsyncValue<void> build() => const AsyncData(null);

  Future<bool> login({
    required String email,
    required String password,
  }) async {
    state = const AsyncLoading();
    try {
      await ref.read(authRepositoryProvider).login(
        email: email,
        password: password,
      );
      state = const AsyncData(null);
      return true;
    } on DioException catch (e) {
      final msg = e.response?.statusCode == 401
          ? 'Invalid email or password'
          : e.message ?? 'Network error';
      state = AsyncError(msg, StackTrace.current);
      return false;
    } on Object catch (e) {
      state = AsyncError(e.toString(), StackTrace.current);
      return false;
    }
  }
}
