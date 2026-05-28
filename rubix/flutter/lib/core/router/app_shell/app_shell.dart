import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';

/// Persistent in-app chrome around /home, /connections, /settings.
///
/// shadcn/ui-flavoured chrome: hairline-bordered frosted top bar with brand
/// mark, animated collapsible sidebar on wide layouts (sliding teal active
/// indicator, spring icon hover, staggered mount), and a custom tab bar on
/// narrow layouts with an animated sliding indicator. No Material
/// [NavigationBar]/[NavigationRail]/[AppBar].
class AppShell extends ConsumerStatefulWidget {
  const AppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  static const double _railBreakpoint = 720;
  static const double _railCollapseBreakpoint = 960;

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  bool? _collapsedOverride;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final activeAsync = ref.watch(activeConnectionProvider);
    final pinAsync = ref.watch(connectionsPinProvider);
    final pinSet = (pinAsync.value ?? '').isNotEmpty;

    final destinations = <_Destination>[
      _Destination(
        icon: LucideIcons.home,
        label: l.home,
        section: 'Overview',
      ),
      const _Destination(
        icon: LucideIcons.layoutDashboard,
        label: 'Dashboards',
        section: 'Overview',
      ),
      _Destination(
        icon: LucideIcons.link,
        label: l.connections,
        section: 'Fleet',
      ),
      _Destination(
        icon: LucideIcons.settings,
        label: l.settings,
        section: 'Platform',
      ),
    ];

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= AppShell._railBreakpoint;
        final autoCollapse =
            constraints.maxWidth < AppShell._railCollapseBreakpoint;
        final collapsed = _collapsedOverride ?? autoCollapse;

        final topBar = _TopBar(
          activeAsync: activeAsync,
          pinSet: pinSet,
          onLock: () => ref.read(pinUnlockedProvider.notifier).lock(),
          onConnectionsTap: () => context.go('/connections'),
          fallbackLabel: l.home,
          onToggleSidebar: wide
              ? () => setState(() => _collapsedOverride = !collapsed)
              : null,
          sidebarCollapsed: collapsed,
          compact: !wide,
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
                    selectedIndex: widget.navigationShell.currentIndex,
                    collapsed: collapsed,
                    onSelected: _go,
                  ),
                  Expanded(
                    child: Column(
                      children: [
                        topBar,
                        Expanded(
                          child: _AnimatedRouteSwitcher(
                            index: widget.navigationShell.currentIndex,
                            child: widget.navigationShell,
                          ),
                        ),
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
          // Let the body paint UNDER the bottom nav so the ambient teal
          // glow can show through the frosted bar (Figma node 6:185).
          extendBody: true,
          body: SafeArea(
            bottom: false,
            child: Column(
              children: [
                topBar,
                Expanded(
                  child: _AnimatedRouteSwitcher(
                    index: widget.navigationShell.currentIndex,
                    child: widget.navigationShell,
                  ),
                ),
              ],
            ),
          ),
          bottomNavigationBar: _TabBar(
            destinations: destinations,
            selectedIndex: widget.navigationShell.currentIndex,
            onSelected: _go,
          ),
        );
      },
    );
  }

  void _go(int index) {
    widget.navigationShell.goBranch(
      index,
      initialLocation: index == widget.navigationShell.currentIndex,
    );
  }
}

class _Destination {
  const _Destination({
    required this.icon,
    required this.label,
    required this.section,
  });
  final IconData icon;
  final String label;
  final String section;
}

// ---------------------------------------------------------------------------
// Animated branch switcher — fade + 8px slide between top-level routes.
// ---------------------------------------------------------------------------
class _AnimatedRouteSwitcher extends StatelessWidget {
  const _AnimatedRouteSwitcher({required this.index, required this.child});

  final int index;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 260),
      switchInCurve: Curves.easeOutCubic,
      switchOutCurve: Curves.easeInCubic,
      transitionBuilder: (child, anim) {
        final offset = Tween<Offset>(
          begin: const Offset(0, 0.012),
          end: Offset.zero,
        ).animate(anim);
        return FadeTransition(
          opacity: anim,
          child: SlideTransition(position: offset, child: child),
        );
      },
      layoutBuilder: (currentChild, previousChildren) => Stack(
        fit: StackFit.expand,
        alignment: Alignment.topLeft,
        children: [
          ...previousChildren,
          if (currentChild != null) Positioned.fill(child: currentChild),
        ],
      ),
      child: KeyedSubtree(key: ValueKey<int>(index), child: child),
    );
  }
}

// ---------------------------------------------------------------------------
// Top bar — frosted, hairline border, animated sidebar toggle.
// ---------------------------------------------------------------------------
class _TopBar extends StatelessWidget {
  const _TopBar({
    required this.activeAsync,
    required this.pinSet,
    required this.onLock,
    required this.onConnectionsTap,
    required this.fallbackLabel,
    required this.onToggleSidebar,
    required this.sidebarCollapsed,
    this.compact = false,
  });

  final AsyncValue<dynamic> activeAsync;
  final bool pinSet;
  final VoidCallback onLock;
  final VoidCallback onConnectionsTap;
  final String fallbackLabel;
  final VoidCallback? onToggleSidebar;
  final bool sidebarCollapsed;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 14, sigmaY: 14),
        child: Container(
          height: 64,
          padding: EdgeInsets.symmetric(horizontal: compact ? 16 : 12),
          decoration: BoxDecoration(
            color: t.bg.withValues(alpha: 0.72),
            border: Border(bottom: BorderSide(color: t.border)),
          ),
          child: compact
              ? _CompactBrandRow(
                  isDark: isDark,
                  leaf: t.leaf,
                  onConnectionsTap: onConnectionsTap,
                  onAccountTap: () {},
                )
              : Row(
                  children: [
                    if (onToggleSidebar != null) ...[
                      _ChromeIconButton(
                        icon: sidebarCollapsed
                            ? LucideIcons.panelLeftOpen
                            : LucideIcons.panelLeftClose,
                        tooltip: sidebarCollapsed
                            ? 'Expand sidebar'
                            : 'Collapse sidebar',
                        onTap: onToggleSidebar!,
                      ),
                      const SizedBox(width: 6),
                    ],
                    Expanded(
                      child: Row(
                        children: [
                          Flexible(
                            child: _CmdKTrigger(
                              onTap: () => _showCommandPalette(context),
                            ),
                          ),
                          const SizedBox(width: 12),
                          Flexible(
                            child: MouseRegion(
                              cursor: SystemMouseCursors.click,
                              child: GestureDetector(
                                behavior: HitTestBehavior.opaque,
                                onTap: onConnectionsTap,
                                child: activeAsync.maybeWhen(
                                  data: (conn) => conn == null
                                      ? _FallbackTitle(label: fallbackLabel)
                                      : _ConnectionLabel(
                                          label: (conn as dynamic).label
                                              as String,
                                          baseUrl: (conn as dynamic).baseUrl
                                              as String,
                                        ),
                                  orElse: () =>
                                      _FallbackTitle(label: fallbackLabel),
                                ),
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (pinSet)
                      _ChromeIconButton(
                        icon: LucideIcons.lock,
                        tooltip: 'Lock',
                        onTap: onLock,
                      ),
                    const SizedBox(width: 4),
                    _ChromeIconButton(
                      icon: LucideIcons.bell,
                      tooltip: 'Notifications',
                      onTap: () {},
                    ),
                    const SizedBox(width: 4),
                    _ChromeIconButton(
                      icon: LucideIcons.user,
                      tooltip: 'Account',
                      onTap: () {},
                    ),
                  ],
                ),
        ).animate().fadeIn(duration: 220.ms, curve: Curves.easeOutCubic),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Compact (mobile) top bar: brand mark + Rubix / IoT Console + user avatar.
// Matches the Figma reference for the Rubix app.
// ---------------------------------------------------------------------------
class _CompactBrandRow extends StatelessWidget {
  const _CompactBrandRow({
    required this.isDark,
    required this.leaf,
    required this.onConnectionsTap,
    required this.onAccountTap,
  });

  final bool isDark;
  final Color leaf;
  final VoidCallback onConnectionsTap;
  final VoidCallback onAccountTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Row(
      children: [
        _BrandSquare(leaf: leaf, isDark: isDark),
        const SizedBox(width: 12),
        Expanded(
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: onConnectionsTap,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'Rubix',
                  style: TextStyle(
                    color: t.text,
                    fontSize: 17,
                    fontWeight: FontWeight.w600,
                    letterSpacing: -0.3,
                    height: 1.1,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  'IoT Console',
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 12,
                    height: 1.1,
                  ),
                ),
              ],
            ),
          ),
        ),
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: onAccountTap,
          child: Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: t.surface2,
              shape: BoxShape.circle,
              border: Border.all(color: t.border),
            ),
            alignment: Alignment.center,
            child: Text(
              'L',
              style: TextStyle(
                color: t.text,
                fontSize: 13,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _BrandSquare extends StatelessWidget {
  const _BrandSquare({required this.leaf, required this.isDark});
  final Color leaf;
  final bool isDark;
  @override
  Widget build(BuildContext context) {
    return Container(
      width: 36,
      height: 36,
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [leaf, Color.lerp(leaf, Colors.black, 0.20)!],
        ),
        borderRadius: BorderRadius.circular(10),
        boxShadow: [
          BoxShadow(
            color: leaf.withValues(alpha: 0.28),
            blurRadius: 14,
            spreadRadius: -2,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      alignment: Alignment.center,
      child: Text(
        'R',
        style: TextStyle(
          color: isDark ? const Color(0xFF0A0A0A) : Colors.white,
          fontWeight: FontWeight.w700,
          fontSize: 16,
          height: 1,
        ),
      ),
    );
  }
}

void _showCommandPalette(BuildContext context) {
  // Placeholder — wire up to a real palette later.
  ScaffoldMessenger.of(context).showSnackBar(
    const SnackBar(
      content: Text('Command palette — coming soon'),
      duration: Duration(milliseconds: 1400),
    ),
  );
}

class _CmdKTrigger extends StatefulWidget {
  const _CmdKTrigger({required this.onTap});
  final VoidCallback onTap;

  @override
  State<_CmdKTrigger> createState() => _CmdKTriggerState();
}

class _CmdKTriggerState extends State<_CmdKTrigger> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 160),
          curve: Curves.easeOutCubic,
          constraints: const BoxConstraints(maxWidth: 280, minHeight: 34),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          decoration: BoxDecoration(
            color: _hover ? t.surface2 : t.surface,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: t.border),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(LucideIcons.search, size: 14, color: t.muted),
              const SizedBox(width: 8),
              Text(
                'Search…',
                style: TextStyle(
                  color: t.muted,
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(width: 16),
              const NubeKbd('⌘K'),
            ],
          ),
        ),
      ),
    );
  }
}

class _BrandMark extends StatelessWidget {
  const _BrandMark({required this.isDark, required this.leaf});
  final bool isDark;
  final Color leaf;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 28,
      height: 28,
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [leaf, Color.lerp(leaf, Colors.black, 0.18)!],
        ),
        borderRadius: BorderRadius.circular(7),
        boxShadow: [
          BoxShadow(
            color: leaf.withValues(alpha: 0.32),
            blurRadius: 14,
            spreadRadius: -2,
            offset: const Offset(0, 2),
          ),
        ],
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
        letterSpacing: -0.2,
      ),
    ).animate().fadeIn(duration: 240.ms).slideX(
          begin: -0.06,
          end: 0,
          duration: 240.ms,
          curve: Curves.easeOutCubic,
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
            letterSpacing: -0.1,
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
    ).animate().fadeIn(duration: 240.ms).slideX(
          begin: -0.04,
          end: 0,
          duration: 240.ms,
          curve: Curves.easeOutCubic,
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
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final scale = _press ? 0.92 : (_hover ? 1.04 : 1.0);
    final btn = MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() {
        _hover = false;
        _press = false;
      }),
      child: Listener(
        onPointerDown: (_) => setState(() => _press = true),
        onPointerUp: (_) => setState(() => _press = false),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: widget.onTap,
          child: AnimatedScale(
            scale: scale,
            duration: const Duration(milliseconds: 140),
            curve: Curves.easeOutCubic,
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 160),
              curve: Curves.easeOutCubic,
              width: 36,
              height: 36,
              decoration: BoxDecoration(
                color: _hover ? t.surface2 : Colors.transparent,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: _hover ? t.border : Colors.transparent,
                ),
              ),
              alignment: Alignment.center,
              child: Icon(widget.icon, size: 18, color: t.muted),
            ),
          ),
        ),
      ),
    );
    final tooltip = widget.tooltip;
    return tooltip != null
        ? Tooltip(message: tooltip, child: btn)
        : btn;
  }
}

// ---------------------------------------------------------------------------
// Sidebar (wide layout) — workspace header, sectioned nav, footer user chip.
// ---------------------------------------------------------------------------
class _Sidebar extends StatelessWidget {
  const _Sidebar({
    required this.destinations,
    required this.selectedIndex,
    required this.collapsed,
    required this.onSelected,
  });

  final List<_Destination> destinations;
  final int selectedIndex;
  final bool collapsed;
  final ValueChanged<int> onSelected;

  static const double _itemHeight = 38;
  static const double _itemGap = 4;
  static const double _vPad = 12;
  static const double _expandedWidth = 240;
  static const double _collapsedWidth = 64;
  static const double _sectionHeight = 22;
  static const double _sectionTopGap = 14;
  static const double _sectionBottomGap = 8;

  /// Computes the top-Y inside the nav stack for each destination so the
  /// sliding pill can land on the correct row, accounting for section
  /// headers when expanded.
  List<double> _itemTops() {
    final tops = <double>[];
    double y = 4; // matches the leading SizedBox(height: 4)
    String? lastSection;
    for (var i = 0; i < destinations.length; i++) {
      final d = destinations[i];
      if (!collapsed && d.section != lastSection) {
        if (lastSection != null) y += _sectionTopGap;
        y += _sectionHeight + _sectionBottomGap;
        lastSection = d.section;
      }
      tops.add(y);
      y += _itemHeight + _itemGap;
    }
    return tops;
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final tops = _itemTops();
    final pillTop = tops[selectedIndex];

    return AnimatedContainer(
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
      width: collapsed ? _collapsedWidth : _expandedWidth,
      decoration: BoxDecoration(
        color: t.surface,
        border: Border(right: BorderSide(color: t.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _WorkspaceHeader(collapsed: collapsed, isDark: isDark),
          Container(height: 1, color: t.border),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: 10,
                vertical: _vPad,
              ),
              child: Stack(
                children: [
                  // Sliding teal accent bar on the left edge.
                  AnimatedPositioned(
                    duration: const Duration(milliseconds: 320),
                    curve: Curves.easeOutCubic,
                    top: pillTop + 9,
                    left: -10,
                    child: Container(
                      width: 3,
                      height: _itemHeight - 18,
                      decoration: BoxDecoration(
                        color: t.leaf,
                        borderRadius: const BorderRadius.only(
                          topRight: Radius.circular(2),
                          bottomRight: Radius.circular(2),
                        ),
                        boxShadow: [
                          BoxShadow(
                            color: t.leaf.withValues(alpha: 0.45),
                            blurRadius: 10,
                            spreadRadius: -1,
                          ),
                        ],
                      ),
                    ),
                  ),
                  // Sliding background pill — animates between selected items.
                  AnimatedPositioned(
                    duration: const Duration(milliseconds: 320),
                    curve: Curves.easeOutCubic,
                    top: pillTop,
                    left: 0,
                    right: 0,
                    height: _itemHeight,
                    child: Container(
                      decoration: BoxDecoration(
                        color: isDark
                            ? const Color(0xFF142F33)
                            : const Color(0xFFF2F7F7),
                        borderRadius: BorderRadius.circular(8),
                      ),
                    ),
                  ),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      const SizedBox(height: 4),
                      for (var i = 0; i < destinations.length; i++) ...[
                        if (!collapsed &&
                            (i == 0 ||
                                destinations[i].section !=
                                    destinations[i - 1].section)) ...[
                          if (i != 0) const SizedBox(height: _sectionTopGap),
                          Padding(
                            padding: const EdgeInsets.only(
                              left: 4,
                              bottom: _sectionBottomGap,
                            ),
                            child: SizedBox(
                              height: _sectionHeight,
                              child: Align(
                                alignment: Alignment.centerLeft,
                                child: NubeEyebrow(destinations[i].section),
                              ),
                            ),
                          ),
                        ],
                        _NavItem(
                          destination: destinations[i],
                          selected: i == selectedIndex,
                          collapsed: collapsed,
                          height: _itemHeight,
                          onTap: () => onSelected(i),
                        ),
                        const SizedBox(height: _itemGap),
                      ],
                    ],
                  ),
                ],
              ),
            ),
          ),
          Container(height: 1, color: t.border),
          _UserFooter(collapsed: collapsed),
        ],
      ),
    );
  }
}

class _WorkspaceHeader extends StatelessWidget {
  const _WorkspaceHeader({required this.collapsed, required this.isDark});
  final bool collapsed;
  final bool isDark;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 14, 10, 14),
      child: Row(
        children: [
          _BrandMark(isDark: isDark, leaf: t.leaf),
          if (!collapsed) ...[
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Nube',
                    style: TextStyle(
                      color: t.text,
                      fontSize: 13.5,
                      fontWeight: FontWeight.w600,
                      letterSpacing: -0.1,
                      height: 1.15,
                    ),
                  ),
                  Text(
                    'IoT Console',
                    style: TextStyle(
                      color: t.muted,
                      fontSize: 11.5,
                      fontWeight: FontWeight.w500,
                      height: 1.2,
                    ),
                  ),
                ],
              ),
            ),
            Icon(LucideIcons.chevronsUpDown, size: 14, color: t.muted),
          ],
        ],
      ),
    );
  }
}

class _UserFooter extends StatelessWidget {
  const _UserFooter({required this.collapsed});
  final bool collapsed;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 12, 10, 12),
      child: Row(
        children: [
          Container(
            width: 30,
            height: 30,
            decoration: BoxDecoration(
              color: t.leaf.withValues(alpha: 0.18),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: t.leaf.withValues(alpha: 0.30)),
            ),
            alignment: Alignment.center,
            child: Text(
              'OP',
              style: TextStyle(
                color: t.leaf,
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.4,
              ),
            ),
          ),
          if (!collapsed) ...[
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Operator',
                    style: TextStyle(
                      color: t.text,
                      fontSize: 12.5,
                      fontWeight: FontWeight.w600,
                      height: 1.15,
                    ),
                  ),
                  Text(
                    'ops@nube.io',
                    style: TextStyle(
                      color: t.muted,
                      fontSize: 11,
                      fontWeight: FontWeight.w500,
                      height: 1.2,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            Icon(LucideIcons.chevronsUpDown, size: 14, color: t.muted),
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
    required this.collapsed,
    required this.height,
    required this.onTap,
  });
  final _Destination destination;
  final bool selected;
  final bool collapsed;
  final double height;
  final VoidCallback onTap;

  @override
  State<_NavItem> createState() => _NavItemState();
}

class _NavItemState extends State<_NavItem> {
  bool _hover = false;
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final selected = widget.selected;
    final fg = selected ? t.leaf : (_hover ? t.text : t.muted);
    final scale = _press ? 0.97 : 1.0;

    final row = Row(
      children: [
        AnimatedScale(
          scale: selected ? 1.06 : (_hover ? 1.04 : 1.0),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOutBack,
          child: Icon(widget.destination.icon, size: 17, color: fg),
        ),
        Flexible(
          child: ClipRect(
            child: AnimatedAlign(
              alignment: Alignment.centerLeft,
              duration: const Duration(milliseconds: 260),
              curve: Curves.easeOutCubic,
              widthFactor: widget.collapsed ? 0 : 1,
              child: AnimatedOpacity(
                duration: const Duration(milliseconds: 200),
                opacity: widget.collapsed ? 0 : 1,
                child: Padding(
                  padding: const EdgeInsets.only(left: 10),
                  child: Row(
                    children: [
                      Expanded(
                        child: AnimatedDefaultTextStyle(
                          duration: const Duration(milliseconds: 200),
                          style: TextStyle(
                            color: fg,
                            fontSize: 13.5,
                            fontWeight: selected
                                ? FontWeight.w600
                                : FontWeight.w500,
                            letterSpacing: -0.1,
                          ),
                          child: Text(
                            widget.destination.label,
                            maxLines: 1,
                            overflow: TextOverflow.fade,
                            softWrap: false,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ],
    );

    final item = SizedBox(
      height: widget.height,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: AnimatedScale(
          scale: scale,
          duration: const Duration(milliseconds: 120),
          curve: Curves.easeOutCubic,
          child: Align(alignment: Alignment.centerLeft, child: row),
        ),
      ),
    );

    final hitArea = MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() {
        _hover = false;
        _press = false;
      }),
      child: Listener(
        onPointerDown: (_) => setState(() => _press = true),
        onPointerUp: (_) => setState(() => _press = false),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: widget.onTap,
          child: item,
        ),
      ),
    );

    return widget.collapsed
        ? Tooltip(
            message: widget.destination.label,
            waitDuration: const Duration(milliseconds: 300),
            child: hitArea,
          )
        : hitArea;
  }
}

// ---------------------------------------------------------------------------
// Tab bar (narrow layout) — animated sliding indicator + bouncing icon.
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
    // Frosted-glass recipe — Figma node 6:185. White at 6% fill +
    // 1px white-at-8% top hairline so the ambient teal glow reads
    // THROUGH the bar instead of being capped by a solid plate.
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 12, sigmaY: 12),
        child: Container(
          decoration: BoxDecoration(
            color: Colors.white.withValues(alpha: 0.06),
            border: Border(
              top: BorderSide(
                color: Colors.white.withValues(alpha: 0.08),
              ),
            ),
          ),
          padding: EdgeInsets.only(
            top: 4,
            bottom: 4 + MediaQuery.of(context).padding.bottom,
          ),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final n = destinations.length;
              final itemWidth = constraints.maxWidth / n;
              return SizedBox(
                height: 56,
                child: Stack(
                  children: [
                    // Sliding indicator at the top.
                    AnimatedPositioned(
                      duration: const Duration(milliseconds: 320),
                      curve: Curves.easeOutCubic,
                      left: selectedIndex * itemWidth + itemWidth / 2 - 14,
                      top: 4,
                      child: Container(
                        width: 28,
                        height: 3,
                        decoration: BoxDecoration(
                          color: t.leaf,
                          borderRadius: BorderRadius.circular(2),
                          boxShadow: [
                            BoxShadow(
                              color: t.leaf.withValues(alpha: 0.45),
                              blurRadius: 8,
                              spreadRadius: -1,
                            ),
                          ],
                        ),
                      ),
                    ),
                    Row(
                      children: [
                        for (var i = 0; i < n; i++)
                          Expanded(
                            child: _TabItem(
                              destination: destinations[i],
                              selected: i == selectedIndex,
                              onTap: () => onSelected(i),
                            ),
                          ),
                      ],
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _TabItem extends StatefulWidget {
  const _TabItem({
    required this.destination,
    required this.selected,
    required this.onTap,
  });
  final _Destination destination;
  final bool selected;
  final VoidCallback onTap;

  @override
  State<_TabItem> createState() => _TabItemState();
}

class _TabItemState extends State<_TabItem> {
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final fg = widget.selected ? t.leaf : t.muted;
    return Listener(
      onPointerDown: (_) => setState(() => _press = true),
      onPointerUp: (_) => setState(() => _press = false),
      onPointerCancel: (_) => setState(() => _press = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _press ? 0.94 : 1.0,
          duration: const Duration(milliseconds: 120),
          curve: Curves.easeOutCubic,
          child: SizedBox(
            height: 56,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                AnimatedScale(
                  scale: widget.selected ? 1.12 : 1.0,
                  duration: const Duration(milliseconds: 260),
                  curve: Curves.easeOutBack,
                  child: Icon(widget.destination.icon, size: 20, color: fg),
                ),
                const SizedBox(height: 4),
                AnimatedDefaultTextStyle(
                  duration: const Duration(milliseconds: 200),
                  style: TextStyle(
                    color: fg,
                    fontSize: 11,
                    fontWeight:
                        widget.selected ? FontWeight.w600 : FontWeight.w500,
                    letterSpacing: -0.1,
                  ),
                  child: Text(widget.destination.label),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
