import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:provision_app/features/auth/data/auth_controller.dart';
import 'package:provision_app/features/auth/presentation/connect_screen.dart';
import 'package:provision_app/features/devices/presentation/device_detail_screen.dart';
import 'package:provision_app/features/devices/presentation/devices_screen.dart';
import 'package:provision_app/features/home/presentation/home_screen.dart';
import 'package:provision_app/features/preview/presentation/page_preview_screen.dart';
import 'package:provision_app/features/scan/presentation/scan_flow.dart';
import 'package:provision_app/features/sites/presentation/sites_screen.dart';
import 'package:provision_app/features/templates/presentation/templates_screen.dart';
import 'package:provision_app/router/app_shell.dart';

/// The app router. Gates everything behind the Connect screen until the session
/// is authenticated (the Flutter analogue of `App.tsx`'s `!user ? <Connect/>`),
/// then presents the tabbed shell via a [StatefulShellRoute]. The Scan tab
/// deep-links into the Preview tab via `/preview?page=<id>` after a provision.

/// Stable navigator keys, declared at top level so they survive any rebuild of
/// the router provider. Without fixed keys the `StatefulShellRoute` mints fresh
/// GlobalKeys each time the provider is recomputed (e.g. on hot restart), and
/// the old + new shells briefly coexist in the tree — the "Duplicate GlobalKey"
/// error. The root navigator key plus one stable key per branch pins every
/// navigator that go_router would otherwise re-key.
final _rootNavigatorKey = GlobalKey<NavigatorState>(debugLabel: 'root');
final _branchKeys = <GlobalKey<NavigatorState>>[
  GlobalKey<NavigatorState>(debugLabel: 'home'),
  GlobalKey<NavigatorState>(debugLabel: 'devices'),
  GlobalKey<NavigatorState>(debugLabel: 'preview'),
  GlobalKey<NavigatorState>(debugLabel: 'sites'),
  GlobalKey<NavigatorState>(debugLabel: 'scan'),
  GlobalKey<NavigatorState>(debugLabel: 'templates'),
];

/// The single, stable [GoRouter] instance. Built once and cached so neither a
/// provider re-read nor the `_AuthRefreshListenable` ever constructs a second
/// router (which would tear down and re-mount the whole shell).
GoRouter? _router;

final appRouterProvider = Provider<GoRouter>((ref) {
  if (_router != null) return _router!;

  // Bridge auth state into a Listenable so go_router re-evaluates `redirect`
  // (gate in/out) whenever the session changes.
  final refresh = _AuthRefreshListenable(ref);
  ref.onDispose(() {
    refresh.dispose();
    _router = null;
  });

  _router = GoRouter(
    initialLocation: '/home',
    navigatorKey: _rootNavigatorKey,
    refreshListenable: refresh,
    redirect: (context, state) {
      final auth = ref.read(authControllerProvider);
      // Hold on the splash until the boot-time session restore finishes.
      if (!auth.ready) return null;
      final atConnect = state.matchedLocation == '/connect';
      if (!auth.authed) return atConnect ? null : '/connect';
      // Land on Home after auth — no longer straight into the camera.
      if (auth.authed && atConnect) return '/home';
      return null;
    },
    routes: [
      GoRoute(
        path: '/connect',
        builder: (context, state) => const ConnectScreen(),
      ),
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) =>
            AppShell(navigationShell: navigationShell),
        branches: [
          // Branch order MUST match `navPages` in router/pages.dart and the
          // `_branchKeys` order above: home, devices, preview, sites, scan,
          // templates.
          StatefulShellBranch(
            navigatorKey: _branchKeys[0],
            routes: [
              GoRoute(
                path: '/home',
                builder: (context, state) => HomeScreen(
                  // "Start scanning" → scan tab, camera mode.
                  onScan: () => context.go('/scan'),
                  // "Add manually" → scan tab, manual entry.
                  onAddManually: () => context.go('/scan?manual=1'),
                  onOpenDevices: () => context.go('/devices'),
                  onOpenSites: () => context.go('/sites'),
                  onOpenTemplates: () => context.go('/templates'),
                  onOpenDevice: (device) => Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (_) => DeviceDetailScreen(device: device),
                    ),
                  ),
                ),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: _branchKeys[1],
            routes: [
              GoRoute(
                path: '/devices',
                builder: (context, state) => const DevicesScreen(),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: _branchKeys[2],
            routes: [
              GoRoute(
                path: '/preview',
                builder: (context, state) => PagePreviewScreen(
                  initialPageId: state.uri.queryParameters['page'],
                ),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: _branchKeys[3],
            routes: [
              GoRoute(
                path: '/sites',
                builder: (context, state) => const SitesScreen(),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: _branchKeys[4],
            routes: [
              GoRoute(
                path: '/scan',
                builder: (context, state) => ScanFlow(
                  startManual: state.uri.queryParameters['manual'] == '1',
                  onPreview: (pageId) => context.go('/preview?page=$pageId'),
                ),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: _branchKeys[5],
            routes: [
              GoRoute(
                path: '/templates',
                builder: (context, state) => const TemplatesScreen(),
              ),
            ],
          ),
        ],
      ),
    ],
  );
  return _router!;
});

/// Notifies go_router whenever the auth state transitions (ready / authed),
/// so the gate redirect re-runs without watching inside the Provider (which
/// would rebuild the whole GoRouter and tear down the navigator).
class _AuthRefreshListenable extends ChangeNotifier {
  _AuthRefreshListenable(Ref ref) {
    _sub = ref.listen(authControllerProvider, (prev, next) {
      if (prev?.ready != next.ready || prev?.authed != next.authed) {
        notifyListeners();
      }
    });
  }

  late final ProviderSubscription<AuthState> _sub;

  @override
  void dispose() {
    _sub.close();
    super.dispose();
  }
}
