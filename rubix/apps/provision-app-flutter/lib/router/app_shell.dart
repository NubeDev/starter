import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/router/nav_bar.dart';
import 'package:provision_app/router/pages.dart';
import 'package:provision_app/router/top_bar.dart';
import 'package:provision_app/shared/widgets/toast.dart';

/// Persistent in-app chrome around the tabs — the Flutter analogue of the React
/// `Shell` inside `App.tsx`. Layers (bottom → top): the active branch (an
/// indexed stack from go_router), the floating [TopBar], the floating [NavBar]
/// + FAB, and the [ToastHost]. The obsidian base + mood backdrop are global
/// (MaterialApp.router builder). Branch switches cross-fade.
class AppShell extends ConsumerWidget {
  const AppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = ref.watch(lookProvider);
    final activeRoute = primaryAndSecondaryRoute(navigationShell.currentIndex);

    return Scaffold(
      // Transparent — the obsidian base + animated MoodBackdrop are provided
      // globally by MaterialApp.router's builder so the gate shares them.
      backgroundColor: Colors.transparent,
      // The dock + topbar float over the body; let the body extend under them.
      resizeToAvoidBottomInset: false,
      body: Stack(
        children: [
          // Branch content — cross-fade between tabs like the React
          // AnimatePresence page transition.
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 220),
            child: KeyedSubtree(
              key: ValueKey(navigationShell.currentIndex),
              child: navigationShell,
            ),
          ),
          const TopBar(),
          NavBar(
            activeRoute: activeRoute,
            accent: look.accent,
            onSelect: _goRoute,
            onFab: () => _goBranch(0), // Scan is the FAB action
          ),
          const ToastHost(),
        ],
      ),
    );
  }

  /// Map a branch index back to its route, for NavBar highlighting.
  String primaryAndSecondaryRoute(int index) =>
      index >= 0 && index < navPages.length ? navPages[index].route : '/scan';

  void _goRoute(String route) {
    final index = navPages.indexWhere((p) => p.route == route);
    if (index >= 0) _goBranch(index);
  }

  void _goBranch(int index) {
    navigationShell.goBranch(
      index,
      initialLocation: index == navigationShell.currentIndex,
    );
  }
}
