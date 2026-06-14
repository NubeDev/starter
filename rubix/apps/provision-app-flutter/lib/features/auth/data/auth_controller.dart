import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/network/auth_user.dart';
import 'package:provision_app/core/network/network_providers.dart';
import 'package:provision_app/core/network/ping_result.dart';
import 'package:provision_app/core/network/tenant.dart';
import 'package:provision_app/core/theme/statuses.dart';
import 'package:provision_app/core/theme/theme_providers.dart';

/// Session state for the whole app — the Dart port of the React `AuthProvider`.
/// Gates the UI until authenticated, and drives the live connection [StatusKey]
/// that tints the app accent (online when authed, offline when not, pairing
/// while logging in, fault on failure).
@immutable
class AuthState {
  const AuthState({
    this.user,
    this.ready = false,
    this.busy = false,
    this.error,
  });

  /// the signed-in principal, or null
  final AuthUser? user;

  /// the initial me() restore finished
  final bool ready;

  /// a login is in flight
  final bool busy;
  final String? error;

  bool get authed => user != null;

  AuthState copyWith({
    AuthUser? user,
    bool clearUser = false,
    bool? ready,
    bool? busy,
    String? error,
    bool clearError = false,
  }) {
    return AuthState(
      user: clearUser ? null : (user ?? this.user),
      ready: ready ?? this.ready,
      busy: busy ?? this.busy,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

final authControllerProvider =
    NotifierProvider<AuthController, AuthState>(AuthController.new);

class AuthController extends Notifier<AuthState> {
  @override
  AuthState build() => const AuthState();

  void _status(StatusKey? s) => ref.read(statusProvider.notifier).set(s);

  /// Restore any existing session on boot. Called once from `main()` after the
  /// transport is hydrated.
  Future<void> restore() async {
    final user = await ref.read(transportProvider).me();
    _status(user != null ? StatusKey.online : StatusKey.offline);
    state = state.copyWith(
      user: user,
      clearUser: user == null,
      ready: true,
    );
  }

  /// Pre-login reachability check; never throws, the result is the verdict.
  Future<PingResult> ping(String baseUrl) {
    state = state.copyWith(clearError: true);
    return ref.read(transportProvider).ping(baseUrl);
  }

  Future<void> login(
    String baseUrl,
    String email,
    String password, {
    String? tenantId,
  }) async {
    state = state.copyWith(busy: true, clearError: true);
    _status(StatusKey.pairing);
    try {
      final user = await ref.read(transportProvider).login(
            baseUrl,
            email,
            password,
            tenantId: tenantId,
          );
      _status(StatusKey.online);
      state = state.copyWith(user: user, busy: false);
    } on TenantRequiredException {
      // Not a failure — the user must pick an org. Reset status and let the
      // Connect screen drive the picker; don't set an error.
      _status(StatusKey.offline);
      state = state.copyWith(busy: false);
      rethrow;
    } catch (e) {
      _status(StatusKey.fault);
      state = state.copyWith(busy: false, error: _message(e));
      rethrow;
    }
  }

  Future<void> logout() async {
    await ref.read(transportProvider).logout();
    _status(StatusKey.offline);
    state = state.copyWith(clearUser: true);
  }

  String _message(Object e) => e.toString().replaceFirst('Exception: ', '');
}
