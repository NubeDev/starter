import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';

/// Persistent in-app chrome around /home, /connections, /settings.
///
/// Custom non-Material chrome: hairline-bordered top bar with brand mark on
/// the left, custom sidebar on wide layouts (icon + label, teal-tinted
/// selected pill), and a custom tab bar on narrow layouts. No Material
/// [NavigationBar]/[NavigationRail]/[AppBar].
class AppShell extends ConsumerWidget {
  const AppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  static const double _railBreakpoint = 720;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final activeAsync = ref.watch(activeConnectionProvider);
    final pinAsync = ref.watch(connectionsPinProvider);
    final pinSet = (pinAsync.value ?? '').isNotEmpty;

    final destinations = <_Destination>[
      _Destination(icon: LucideIcons.home, label: l.home),
      _Destination(icon: LucideIcons.layoutDashboard, label: 'Dashboards'),
      _Destination(icon: LucideIcons.link, label: l.connections),
      _Destination(icon: LucideIcons.settings, label: l.settings),
    ];

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= _railBreakpoint;

        final topBar = _TopBar(
          activeAsync: activeAsync,
          pinSet: pinSet,
          onLock: () => ref.read(pinUnlockedProvider.notifier).lock(),
          onConnectionsTap: () => context.go('/connections'),
          fallbackLabel: l.home,
        );

        if (wide) {
          return Scaffold(
            backgroundColor: t.bg,
            body: SafeArea(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _Sidebar(
                    destinations: destinations,
                    selectedIndex: navigationShell.currentIndex,
                    onSelected: _go,
                  ),
                  Expanded(
                    child: Column(
                      children: [
                        topBar,
                        Expanded(child: navigationShell),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          );
        }

        return Scaffold(
          backgroundColor: t.bg,
          body: SafeArea(
            child: Column(
              children: [
                topBar,
                Expanded(child: navigationShell),
                _TabBar(
                  destinations: destinations,
                  selectedIndex: navigationShell.currentIndex,
                  onSelected: _go,
                ),
              ],
            ),
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
  const _Destination({required this.icon, required this.label});
  final IconData icon;
  final String label;
}

// ---------------------------------------------------------------------------
// Top bar — flat, hairline border.
// ---------------------------------------------------------------------------
class _TopBar extends StatelessWidget {
  const _TopBar({
    required this.activeAsync,
    required this.pinSet,
    required this.onLock,
    required this.onConnectionsTap,
    required this.fallbackLabel,
  });

  final AsyncValue<dynamic> activeAsync;
  final bool pinSet;
  final VoidCallback onLock;
  final VoidCallback onConnectionsTap;
  final String fallbackLabel;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Container(
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: BoxDecoration(
        color: t.bg,
        border: Border(bottom: BorderSide(color: t.border)),
      ),
      child: Row(
        children: [
          Container(
            width: 28,
            height: 28,
            decoration: BoxDecoration(
              color: t.leaf,
              borderRadius: BorderRadius.circular(7),
            ),
            alignment: Alignment.center,
            child: Text(
              'R',
              style: TextStyle(
                color: isDark ? const Color(0xFF0A0A0A) : Colors.white,
                fontWeight: FontWeight.w700,
                fontSize: 14,
                height: 1,
              ),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: MouseRegion(
              cursor: SystemMouseCursors.click,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: onConnectionsTap,
                child: activeAsync.maybeWhen(
                  data: (conn) => conn == null
                      ? _FallbackTitle(label: fallbackLabel)
                      : _ConnectionLabel(
                          label: (conn as dynamic).label as String,
                          baseUrl: (conn as dynamic).baseUrl as String,
                        ),
                  orElse: () => _FallbackTitle(label: fallbackLabel),
                ),
              ),
            ),
          ),
          if (pinSet)
            _ChromeIconButton(
              icon: LucideIcons.lock,
              tooltip: 'Lock',
              onTap: onLock,
            ),
        ],
      ),
    );
  }
}

class _FallbackTitle extends StatelessWidget {
  const _FallbackTitle({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Text(
      label,
      style: TextStyle(
        color: t.text,
        fontSize: 15,
        fontWeight: FontWeight.w600,
      ),
    );
  }
}

class _ConnectionLabel extends StatelessWidget {
  const _ConnectionLabel({required this.label, required this.baseUrl});
  final String label;
  final String baseUrl;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          label,
          style: TextStyle(
            color: t.text,
            fontSize: 14,
            fontWeight: FontWeight.w600,
            height: 1.2,
          ),
          overflow: TextOverflow.ellipsis,
        ),
        Text(
          baseUrl,
          style: TextStyle(
            color: t.muted,
            fontSize: 12,
            fontWeight: FontWeight.w400,
            height: 1.2,
          ),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }
}

class _ChromeIconButton extends StatefulWidget {
  const _ChromeIconButton({
    required this.icon,
    required this.onTap,
    this.tooltip,
  });
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;

  @override
  State<_ChromeIconButton> createState() => _ChromeIconButtonState();
}

class _ChromeIconButtonState extends State<_ChromeIconButton> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final btn = MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: _hover ? t.surface2 : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
          ),
          alignment: Alignment.center,
          child: Icon(widget.icon, size: 18, color: t.muted),
        ),
      ),
    );
    return widget.tooltip != null
        ? Tooltip(message: widget.tooltip!, child: btn)
        : btn;
  }
}

// ---------------------------------------------------------------------------
// Sidebar (wide layout) — vertical list, teal-tinted pill for selected.
// ---------------------------------------------------------------------------
class _Sidebar extends StatelessWidget {
  const _Sidebar({
    required this.destinations,
    required this.selectedIndex,
    required this.onSelected,
  });

  final List<_Destination> destinations;
  final int selectedIndex;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      width: 220,
      decoration: BoxDecoration(
        color: t.surface,
        border: Border(right: BorderSide(color: t.border)),
      ),
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(height: 4),
          for (var i = 0; i < destinations.length; i++) ...[
            _NavItem(
              destination: destinations[i],
              selected: i == selectedIndex,
              onTap: () => onSelected(i),
            ),
            const SizedBox(height: 2),
          ],
        ],
      ),
    );
  }
}

class _NavItem extends StatefulWidget {
  const _NavItem({
    required this.destination,
    required this.selected,
    required this.onTap,
  });
  final _Destination destination;
  final bool selected;
  final VoidCallback onTap;

  @override
  State<_NavItem> createState() => _NavItemState();
}

class _NavItemState extends State<_NavItem> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final selected = widget.selected;
    final bg = selected
        ? (isDark ? const Color(0xFF142F33) : const Color(0xFFF2F7F7))
        : (_hover ? t.surface2 : Colors.transparent);
    final fg = selected ? t.leaf : t.text;

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            children: [
              Icon(widget.destination.icon, size: 16, color: fg),
              const SizedBox(width: 10),
              Text(
                widget.destination.label,
                style: TextStyle(
                  color: fg,
                  fontSize: 13.5,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Tab bar (narrow layout) — flat, icon + label, no Material rail.
// ---------------------------------------------------------------------------
class _TabBar extends StatelessWidget {
  const _TabBar({
    required this.destinations,
    required this.selectedIndex,
    required this.onSelected,
  });

  final List<_Destination> destinations;
  final int selectedIndex;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      decoration: BoxDecoration(
        color: t.surface,
        border: Border(top: BorderSide(color: t.border)),
      ),
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          for (var i = 0; i < destinations.length; i++)
            Expanded(
              child: _TabItem(
                destination: destinations[i],
                selected: i == selectedIndex,
                onTap: () => onSelected(i),
              ),
            ),
        ],
      ),
    );
  }
}

class _TabItem extends StatelessWidget {
  const _TabItem({
    required this.destination,
    required this.selected,
    required this.onTap,
  });
  final _Destination destination;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final fg = selected ? t.leaf : t.muted;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: SizedBox(
        height: 52,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(destination.icon, size: 20, color: fg),
            const SizedBox(height: 4),
            Text(
              destination.label,
              style: TextStyle(
                color: fg,
                fontSize: 11,
                fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
