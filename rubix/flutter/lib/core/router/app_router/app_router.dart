import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/router/app_shell/app_shell.dart';
import 'package:rubix_flutter/features/auth/data/auth_repository/auth_repository.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_list_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_unlock/connections_unlock_screen.dart';
import 'package:rubix_flutter/features/home/presentation/home_screen.dart';
import 'package:rubix_flutter/features/sdui/presentation/dashboard_list_screen.dart';
import 'package:rubix_flutter/features/sdui/presentation/sdui_page_screen.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';
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
  // Watched so the router rebuilds when auto-login finishes — screens
  // can then re-fetch user-scoped data.
  ref.watch(currentTokenProvider);
  final pinAsync = ref.watch(connectionsPinProvider);
  final pinUnlocked = ref.watch(pinUnlockedProvider);

  return GoRouter(
    initialLocation: '/',
    redirect: (context, state) {
      final hasConnection = active.value != null;
      final location = state.matchedLocation;
      final pin = pinAsync.value;
      final pinSet = pin != null && pin.isNotEmpty;
      final isConnectionsRoute = location == '/connections' ||
          location == '/connections/new';

      // PIN gate: if a PIN is set and the session isn't unlocked,
      // bounce any /connections* hit to the unlock screen — but only
      // when there's already an active connection. First-run users
      // (no connection yet) must reach /connections to add one.
      if (hasConnection && pinSet && !pinUnlocked && isConnectionsRoute) {
        return '/connections/unlock';
      }
      // PIN cleared while on the unlock screen → hop to /connections.
      if (location == '/connections/unlock' && (!pinSet || pinUnlocked)) {
        return '/connections';
      }

      // Root → home. All other navigation is free; screens render
      // their own empty / unreachable state when there's no active
      // connection or token.
      if (location == '/') return '/home';

      return null;
    },
    routes: [
      GoRoute(
        path: '/',
        redirect: (_, __) => '/home',
      ),
      GoRoute(
        path: '/login',
        redirect: (_, __) => '/',
      ),
      GoRoute(
        path: '/connections/new',
        builder: (context, state) => const AddConnectionScreen(),
      ),
      GoRoute(
        path: '/connections/unlock',
        builder: (context, state) => const ConnectionsUnlockScreen(),
      ),
      GoRoute(
        path: '/sdui/:pageRef',
        builder: (context, state) => SduiPageScreen(
          pageRef: state.pathParameters['pageRef']!,
        ),
      ),
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            AppShell(navigationShell: navigationShell),
        branches: [
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/home',
                builder: (context, state) => const HomeScreen(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/dashboards',
                builder: (context, state) => const DashboardListScreen(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/connections',
                builder: (context, state) => const ConnectionsListScreen(),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                builder: (context, state) => const SettingsScreen(),
              ),
            ],
          ),
        ],
      ),
    ],
  );
});
