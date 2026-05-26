import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';

/// Persistent in-app chrome around /home, /connections, /settings.
///
/// Owns the AppBar (active-connection label + logout) and the nav
/// surface — NavigationBar on narrow widths, NavigationRail on wide.
class AppShell extends ConsumerWidget {
  const AppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  static const double _railBreakpoint = 600;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final activeAsync = ref.watch(activeConnectionProvider);
    final pinAsync = ref.watch(connectionsPinProvider);
    final pinSet = (pinAsync.value ?? '').isNotEmpty;

    final destinations = <_Destination>[
      _Destination(icon: Icons.home_outlined, selectedIcon: Icons.home,
          label: l.home),
      _Destination(icon: Icons.link_outlined, selectedIcon: Icons.link,
          label: l.connections),
      _Destination(icon: Icons.settings_outlined, selectedIcon: Icons.settings,
          label: l.settings),
    ];

    final appBar = AppBar(
      titleSpacing: 0,
      title: InkWell(
        onTap: () => context.go('/connections'),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: activeAsync.maybeWhen(
            data: (conn) => conn == null
                ? Text(l.connections)
                : _ConnectionLabel(label: conn.label, baseUrl: conn.baseUrl),
            orElse: () => Text(l.home),
          ),
        ),
      ),
      actions: [
        if (pinSet)
          IconButton(
            icon: const Icon(Icons.lock_outline),
            tooltip: 'Lock',
            onPressed: () {
              ref.read(pinUnlockedProvider.notifier).lock();
            },
          ),
      ],
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= _railBreakpoint;
        return Scaffold(
          appBar: appBar,
          body: wide
              ? Row(
                  children: [
                    NavigationRail(
                      selectedIndex: navigationShell.currentIndex,
                      onDestinationSelected: _go,
                      labelType: NavigationRailLabelType.all,
                      destinations: [
                        for (final d in destinations)
                          NavigationRailDestination(
                            icon: Icon(d.icon),
                            selectedIcon: Icon(d.selectedIcon),
                            label: Text(d.label),
                          ),
                      ],
                    ),
                    const VerticalDivider(width: 1),
                    Expanded(child: navigationShell),
                  ],
                )
              : navigationShell,
          bottomNavigationBar: wide
              ? null
              : NavigationBar(
                  selectedIndex: navigationShell.currentIndex,
                  onDestinationSelected: _go,
                  destinations: [
                    for (final d in destinations)
                      NavigationDestination(
                        icon: Icon(d.icon),
                        selectedIcon: Icon(d.selectedIcon),
                        label: d.label,
                      ),
                  ],
                ),
        );
      },
    );
  }

  void _go(int index) {
    navigationShell.goBranch(
      index,
      initialLocation: index == navigationShell.currentIndex,
    );
  }
}

class _Destination {
  _Destination({
    required this.icon,
    required this.selectedIcon,
    required this.label,
  });
  final IconData icon;
  final IconData selectedIcon;
  final String label;
}

class _ConnectionLabel extends StatelessWidget {
  const _ConnectionLabel({required this.label, required this.baseUrl});
  final String label;
  final String baseUrl;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          label,
          style: theme.textTheme.titleMedium,
          overflow: TextOverflow.ellipsis,
        ),
        Text(
          baseUrl,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }
}
