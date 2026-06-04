import 'package:flutter/widgets.dart';
import 'package:lucide_icons/lucide_icons.dart';

/// The app's tab registry — single source of truth for the nav, mirroring the
/// React `pages/registry.tsx`. The NavBar renders by mapping over [navPages];
/// `secondary` pages live in the overflow rail. The branch order here MUST
/// match the `StatefulShellBranch` order in the router.
enum NavGroup { primary, secondary }

@immutable
class NavPage {
  const NavPage({
    required this.route,
    required this.label,
    required this.icon,
    required this.group,
  });

  final String route;
  final String label;
  final IconData icon;
  final NavGroup group;
}

// Order MUST match the StatefulShellBranch order in app_router.dart and the
// `_branchKeys` order. Home is the landing branch (index 0); Scan no longer
// holds a dock slot — the center FAB is its fast action — so it lives in the
// secondary/overflow group while staying a real branch.
const navPages = <NavPage>[
  NavPage(
    route: '/home',
    label: 'Home',
    icon: LucideIcons.home,
    group: NavGroup.primary,
  ),
  NavPage(
    route: '/devices',
    label: 'Devices',
    icon: LucideIcons.cpu,
    group: NavGroup.primary,
  ),
  NavPage(
    route: '/preview',
    label: 'Preview',
    icon: LucideIcons.layoutDashboard,
    group: NavGroup.primary,
  ),
  NavPage(
    route: '/sites',
    label: 'Sites',
    icon: LucideIcons.building2,
    group: NavGroup.primary,
  ),
  NavPage(
    route: '/scan',
    label: 'Scan',
    icon: LucideIcons.scanLine,
    group: NavGroup.secondary,
  ),
  NavPage(
    route: '/templates',
    label: 'Templates',
    icon: LucideIcons.fileCode2,
    group: NavGroup.secondary,
  ),
];

List<NavPage> get primaryPages =>
    navPages.where((p) => p.group == NavGroup.primary).toList();
List<NavPage> get secondaryPages =>
    navPages.where((p) => p.group == NavGroup.secondary).toList();
