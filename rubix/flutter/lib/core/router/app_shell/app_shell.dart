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

  /// Width at which the layout switches from floating pill (mobile) to
  /// 80px side rail (desktop). Deliberate deviation from Figma 1024px.
  static const double _pillBreakpoint = 1024;

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
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
        final wide = constraints.maxWidth >= AppShell._pillBreakpoint;

        // ── Desktop (≥ 1024px): 80px frosted side rail + content ──────────
        if (wide) {
          final topBar = _TopBar(
            activeAsync: activeAsync,
            pinSet: pinSet,
            onLock: () => ref.read(pinUnlockedProvider.notifier).lock(),
            onConnectionsTap: () => context.go('/connections'),
            fallbackLabel: l.home,
            onToggleSidebar: null,
            sidebarCollapsed: false,
            compact: false,
          );
          return Scaffold(
            backgroundColor: t.bg,
            body: SafeArea(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _SideRailNav(
                    destinations: destinations,
                    selectedIndex: widget.navigationShell.currentIndex,
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

        // ── Mobile (< 1024px): compact top bar + floating pill nav ────────
        final topBar = _TopBar(
          activeAsync: activeAsync,
          pinSet: pinSet,
          onLock: () => ref.read(pinUnlockedProvider.notifier).lock(),
          onConnectionsTap: () => context.go('/connections'),
          fallbackLabel: l.home,
          onToggleSidebar: null,
          sidebarCollapsed: false,
          compact: true,
        );
        return Scaffold(
          backgroundColor: t.bg,
          body: SafeArea(
            bottom: false,
            child: Stack(
              children: [
                // Push content up so scrollables clear the floating pill nav.
                // Pill footprint: _pillHeight(68) + _overhang(7) + gap(16) = 91.
                MediaQuery(
                  data: MediaQuery.of(context).copyWith(
                    padding: MediaQuery.of(context).padding.copyWith(
                      bottom: MediaQuery.of(context).padding.bottom + 91,
                    ),
                  ),
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
                // Floating pill nav overlay — overhangs safe area.
                Positioned(
                  left: 16,
                  right: 16,
                  bottom:
                      16 + MediaQuery.of(context).padding.bottom,
                  child: _FloatingPillNav(
                    destinations: destinations,
                    selectedIndex: widget.navigationShell.currentIndex,
                    onSelected: _go,
                  ),
                ),
              ],
            ),
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
// Animated branch switcher — fade-only cross-fade between top-level routes.
// 150ms, no slide, no scale. Static everywhere else.
// ---------------------------------------------------------------------------
class _AnimatedRouteSwitcher extends StatelessWidget {
  const _AnimatedRouteSwitcher({required this.index, required this.child});

  final int index;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 150),
      transitionBuilder: (child, anim) =>
          FadeTransition(opacity: anim, child: child),
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
// Press-scale \u2014 wraps any tap target; 0.96 on press, 1.0 on release, 100ms.
// No Material ripple \u2014 NoSplash is already set globally in AppTheme.
// ---------------------------------------------------------------------------
class _PressScale extends StatefulWidget {
  const _PressScale({required this.child, required this.onTap});

  final Widget child;
  final VoidCallback onTap;

  @override
  State<_PressScale> createState() => _PressScaleState();
}

class _PressScaleState extends State<_PressScale> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerDown: (_) => setState(() => _pressed = true),
      onPointerUp: (_) => setState(() => _pressed = false),
      onPointerCancel: (_) => setState(() => _pressed = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedScale(
          scale: _pressed ? 0.96 : 1.0,
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOutCubic,
          child: widget.child,
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Floating pill nav (mobile, < 1024px).
//
// \u2022 68px tall frosted glass pill, 16px from screen edges.
// \u2022 Active icon inside a 44px teal circle whose top overhangs the pill by 7px.
// \u2022 Inactive icons: 22px lucide outline, muted teal-grey.
// \u2022 Active circle slides between positions with AnimatedPositioned 250ms
//   easeOutCubic.
// \u2022 No labels, no Material indicators.
// ---------------------------------------------------------------------------
class _FloatingPillNav extends StatelessWidget {
  const _FloatingPillNav({
    required this.destinations,
    required this.selectedIndex,
    required this.onSelected,
  });

  final List<_Destination> destinations;
  final int selectedIndex;
  final ValueChanged<int> onSelected;

  static const double _pillHeight = 68;
  static const double _circleSize = 44;
  static const double _overhang = 7;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return LayoutBuilder(
      builder: (context, constraints) {
        final n = destinations.length;
        final slotWidth = constraints.maxWidth / n;
        final circleLeft =
            selectedIndex * slotWidth + slotWidth / 2 - _circleSize / 2;

        return SizedBox(
          height: _pillHeight + _overhang,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              // ─ Frosted glass pill ─────────────────────────────────────────────────
              Positioned(
                left: 0,
                right: 0,
                top: _overhang,
                bottom: 0,
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(_pillHeight / 2),
                  child: BackdropFilter(
                    filter: ImageFilter.blur(sigmaX: 12, sigmaY: 12),
                    child: Container(
                      height: _pillHeight,
                      decoration: BoxDecoration(
                        color: Colors.white.withValues(alpha: 0.06),
                        borderRadius:
                            BorderRadius.circular(_pillHeight / 2),
                        border: Border.all(
                          color: Colors.white.withValues(alpha: 0.08),
                        ),
                      ),
                      child: Row(
                        children: [
                          for (var i = 0; i < n; i++)
                            Expanded(
                              child: _PressScale(
                                onTap: () => onSelected(i),
                                child: Center(
                                  child: AnimatedOpacity(
                                    duration:
                                        const Duration(milliseconds: 150),
                                    opacity: i == selectedIndex ? 0 : 1,
                                    child: Icon(
                                      destinations[i].icon,
                                      size: 22,
                                      color: t.muted,
                                    ),
                                  ),
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
              // ─ Active teal circle (overhangs top by _overhang px) ────────────────────
              AnimatedPositioned(
                duration: const Duration(milliseconds: 250),
                curve: Curves.easeOutCubic,
                left: circleLeft,
                top: 0,
                child: _PressScale(
                  onTap: () => onSelected(selectedIndex),
                  child: Container(
                    width: _circleSize,
                    height: _circleSize,
                    decoration: BoxDecoration(
                      color: t.leaf,
                      shape: BoxShape.circle,
                    ),
                    alignment: Alignment.center,
                    child: Icon(
                      destinations[selectedIndex].icon,
                      size: 22,
                      color: const Color(0xFF0A1A1A),
                    ),
                  ),
                ),
              ),
            ],
          ),
        );
      },
    );
  }
}

// ---------------------------------------------------------------------------
// Desktop side rail (≥ 1024px).
//
// • 80px wide, full screen height, frosted glass, hairline right border.
// • 4 icons evenly distributed vertically.
// • Active: 44px teal circle centred in rail — no overhang.
// • Active circle slides with AnimatedPositioned 250ms easeOutCubic.
// • Tooltip on each icon (label shown on hover).
// ---------------------------------------------------------------------------
class _SideRailNav extends StatelessWidget {
  const _SideRailNav({
    required this.destinations,
    required this.selectedIndex,
    required this.onSelected,
  });

  final List<_Destination> destinations;
  final int selectedIndex;
  final ValueChanged<int> onSelected;

  static const double _railWidth = 80;
  static const double _circleSize = 44;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return ClipRect(
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 12, sigmaY: 12),
        child: Container(
          width: _railWidth,
          decoration: BoxDecoration(
            color: Colors.white.withValues(alpha: 0.06),
            border: Border(
              right: BorderSide(
                color: Colors.white.withValues(alpha: 0.08),
              ),
            ),
          ),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final totalHeight = constraints.maxHeight;
              final n = destinations.length;
              final slotHeight = totalHeight / n;
              final circleTop = selectedIndex * slotHeight +
                  slotHeight / 2 -
                  _circleSize / 2;

              return Stack(
                children: [
                  // ─ Animated active circle ─────────────────────────────────────
                  AnimatedPositioned(
                    duration: const Duration(milliseconds: 250),
                    curve: Curves.easeOutCubic,
                    top: circleTop,
                    left: _railWidth / 2 - _circleSize / 2,
                    child: Container(
                      width: _circleSize,
                      height: _circleSize,
                      decoration: BoxDecoration(
                        color: t.leaf,
                        shape: BoxShape.circle,
                      ),
                      alignment: Alignment.center,
                      child: Icon(
                        destinations[selectedIndex].icon,
                        size: 22,
                        color: const Color(0xFF0A1A1A),
                      ),
                    ),
                  ),
                  // ─ Tappable icon slots ─────────────────────────────────────────
                  Column(
                    children: [
                      for (var i = 0; i < n; i++)
                        Expanded(
                          child: Tooltip(
                            message: destinations[i].label,
                            preferBelow: false,
                            waitDuration: const Duration(milliseconds: 400),
                            child: _PressScale(
                              onTap: () => onSelected(i),
                              child: SizedBox.expand(
                                child: Center(
                                  child: AnimatedOpacity(
                                    duration:
                                        const Duration(milliseconds: 150),
                                    opacity: i == selectedIndex ? 0 : 1,
                                    child: Icon(
                                      destinations[i].icon,
                                      size: 22,
                                      color: t.muted,
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}
