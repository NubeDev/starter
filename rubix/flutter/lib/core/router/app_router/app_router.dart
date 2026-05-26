import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/features/auth/data/auth_repository/auth_repository.dart';
import 'package:rubix_flutter/features/auth/presentation/login/login_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_list_screen.dart';
import 'package:rubix_flutter/features/home/presentation/home_screen.dart';
import 'package:rubix_flutter/features/settings/presentation/settings_screen.dart';

/// Stores the route to restore after re-login (401 eviction).
final pendingRouteProvider =
    NotifierProvider<PendingRouteNotifier, String?>(PendingRouteNotifier.new);

class PendingRouteNotifier extends Notifier<String?> {
  @override
  String? build() => null;

  // ignore: use_setters_to_change_properties
  void set(String? value) => state = value;
}

final appRouterProvider = Provider<GoRouter>((ref) {
  final active = ref.watch(activeConnectionProvider);
  final token = ref.watch(currentTokenProvider);

  return GoRouter(
    initialLocation: '/',
    redirect: (context, state) {
      final hasConnection = active.value != null;
      final hasToken = token.value != null;
      final location = state.matchedLocation;

      // No active connection → must pick/add one.
      if (!hasConnection) {
        if (location == '/connections' || location == '/connections/new') {
          return null;
        }
        return '/connections';
      }

      // Has connection but no token → login.
      if (!hasToken) {
        if (location == '/login' ||
            location == '/connections' ||
            location == '/connections/new') {
          return null;
        }
        // Save where user was heading for post-login restore.
        if (location != '/') {
          ref.read(pendingRouteProvider.notifier).set(location);
        }
        return '/login';
      }

      // Has token, on login page → go home or restore pending.
      if (location == '/login' || location == '/') {
        final pending = ref.read(pendingRouteProvider);
        if (pending != null) {
          ref.read(pendingRouteProvider.notifier).set(null);
          return pending;
        }
        return '/home';
      }

      return null;
    },
    routes: [
      GoRoute(
        path: '/',
        redirect: (_, __) => '/home',
      ),
      GoRoute(
        path: '/home',
        builder: (context, state) => const HomeScreen(),
      ),
      GoRoute(
        path: '/login',
        builder: (context, state) => const LoginScreen(),
      ),
      GoRoute(
        path: '/connections',
        builder: (context, state) => const ConnectionsListScreen(),
      ),
      GoRoute(
        path: '/connections/new',
        builder: (context, state) => const AddConnectionScreen(),
      ),
      GoRoute(
        path: '/settings',
        builder: (context, state) => const SettingsScreen(),
      ),
    ],
  );
});
