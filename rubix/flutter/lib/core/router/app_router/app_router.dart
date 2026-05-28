import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/router/app_shell/app_shell.dart';
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
  // Intentionally NOT watching authControllerProvider. A new auth
  // state would rebuild the GoRouter, which swaps MaterialApp's
  // routerConfig and tears down the navigator — that disposes every
  // autoDispose provider in the active screen and re-runs them on
  // remount, racing with the just-reissued token. Screens that care
  // about auth transitions watch authControllerProvider themselves.
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
        pageBuilder: (context, state) => _fadeScalePage(
          state,
          SduiPageScreen(pageRef: state.pathParameters['pageRef']!),
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
                pageBuilder: (context, state) =>
                    _shellFadePage(state, const HomeScreen()),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/dashboards',
                pageBuilder: (context, state) =>
                    _shellFadePage(state, const DashboardListScreen()),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/connections',
                pageBuilder: (context, state) =>
                    _shellFadePage(state, const ConnectionsListScreen()),
              ),
            ],
          ),
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/settings',
                pageBuilder: (context, state) =>
                    _shellFadePage(state, const SettingsScreen()),
              ),
            ],
          ),
        ],
      ),
    ],
  );
});

// ---------------------------------------------------------------------------
// Page transitions — Framer-Motion-ish fade + tiny slide/scale. The shell's
// branch switcher handles tab swaps; these handle pushed routes (sdui, etc.)
// and the initial branch render.
// ---------------------------------------------------------------------------

/// Subtle fade + 4px upward slide for in-shell branch screens.
CustomTransitionPage<void> _shellFadePage(GoRouterState state, Widget child) {
  return CustomTransitionPage<void>(
    key: state.pageKey,
    child: child,
    transitionDuration: const Duration(milliseconds: 240),
    reverseTransitionDuration: const Duration(milliseconds: 180),
    transitionsBuilder: (context, animation, secondaryAnimation, c) {
      final curved = CurvedAnimation(
        parent: animation,
        curve: Curves.easeOutCubic,
        reverseCurve: Curves.easeInCubic,
      );
      return FadeTransition(
        opacity: curved,
        child: SlideTransition(
          position: Tween<Offset>(
            begin: const Offset(0, 0.012),
            end: Offset.zero,
          ).animate(curved),
          child: c,
        ),
      );
    },
  );
}

/// Fade + slight scale-in for pushed top-level pages (sdui details).
CustomTransitionPage<void> _fadeScalePage(GoRouterState state, Widget child) {
  return CustomTransitionPage<void>(
    key: state.pageKey,
    child: child,
    transitionDuration: const Duration(milliseconds: 280),
    reverseTransitionDuration: const Duration(milliseconds: 200),
    transitionsBuilder: (context, animation, secondaryAnimation, c) {
      final curved = CurvedAnimation(
        parent: animation,
        curve: Curves.easeOutCubic,
        reverseCurve: Curves.easeInCubic,
      );
      return FadeTransition(
        opacity: curved,
        child: ScaleTransition(
          scale: Tween<double>(begin: 0.985, end: 1).animate(curved),
          child: c,
        ),
      );
    },
  );
}
