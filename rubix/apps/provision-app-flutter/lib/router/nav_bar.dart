import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/router/pages.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// Registry-driven floating bottom nav with a center FAB — ported from the
/// React `NavBar`. The icons split evenly around the FAB (which jumps to Scan,
/// the app's fast action); secondary pages live in an overflow rail that
/// springs up above the dock.
class NavBar extends StatefulWidget {
  const NavBar({
    required this.activeRoute,
    required this.accent,
    required this.onSelect,
    required this.onFab,
    super.key,
  });

  final String activeRoute;
  final Color accent;
  final ValueChanged<String> onSelect;
  final VoidCallback onFab;

  @override
  State<NavBar> createState() => _NavBarState();
}

class _NavBarState extends State<NavBar> {
  bool _overflowOpen = false;

  @override
  Widget build(BuildContext context) {
    final primary = primaryPages;
    final secondary = secondaryPages;

    // Split the primary icons evenly around the center FAB so it stays visually
    // centered — independent of how many secondary (overflow) items exist.
    final leftCount = primary.length ~/ 2;
    final left = primary.take(leftCount).toList();
    final right = primary.skip(leftCount).toList();

    void go(String route) {
      widget.onSelect(route);
      setState(() => _overflowOpen = false);
    }

    return Positioned(
      left: 0,
      right: 0,
      bottom: MediaQuery.of(context).padding.bottom + 24,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // overflow rail
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 220),
            transitionBuilder: (child, anim) => FadeTransition(
              opacity: anim,
              child: ScaleTransition(scale: anim, child: child),
            ),
            child: _overflowOpen && secondary.isNotEmpty
                ? Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: GlassSurface(
                      strong: true,
                      borderRadius: BorderRadius.circular(999),
                      boxShadow: GlassShadow.glass,
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          for (final p in secondary)
                            _NavButton(
                              page: p,
                              active: widget.activeRoute == p.route,
                              accent: widget.accent,
                              onTap: () => go(p.route),
                            ),
                        ],
                      ),
                    ),
                  )
                : const SizedBox.shrink(),
          ),
          GlassSurface(
            strong: true,
            borderRadius: BorderRadius.circular(999),
            boxShadow: GlassShadow.glass,
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                for (final p in left)
                  _NavButton(
                    page: p,
                    active: widget.activeRoute == p.route,
                    accent: widget.accent,
                    onTap: () => go(p.route),
                  ),
                _Fab(accent: widget.accent, onTap: widget.onFab),
                for (final p in right)
                  _NavButton(
                    page: p,
                    active: widget.activeRoute == p.route,
                    accent: widget.accent,
                    onTap: () => go(p.route),
                  ),
                if (secondary.isNotEmpty)
                  _NavButton(
                    page: const NavPage(
                      route: '__more__',
                      label: 'More',
                      icon: LucideIcons.moreHorizontal,
                      group: NavGroup.secondary,
                    ),
                    active: _overflowOpen ||
                        secondary.any((p) => p.route == widget.activeRoute),
                    accent: widget.accent,
                    onTap: () => setState(() => _overflowOpen = !_overflowOpen),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _NavButton extends StatelessWidget {
  const _NavButton({
    required this.page,
    required this.active,
    required this.accent,
    required this.onTap,
  });

  final NavPage page;
  final bool active;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return Pressable(
      onTap: onTap,
      scale: 0.88,
      semanticLabel: page.label,
      child: SizedBox(
        width: 48,
        height: 48,
        child: Stack(
          alignment: Alignment.center,
          children: [
            Icon(
              page.icon,
              size: 24,
              color: active ? accent : look.inkMuted,
            ),
            if (active)
              Positioned(
                bottom: 6,
                child: Container(
                  width: 4,
                  height: 4,
                  decoration: BoxDecoration(color: accent, shape: BoxShape.circle),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _Fab extends StatelessWidget {
  const _Fab({required this.accent, required this.onTap});
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: Pressable(
        onTap: onTap,
        scale: 0.9,
        semanticLabel: 'Add a device',
        child: Container(
          width: 56,
          height: 56,
          decoration: BoxDecoration(
            color: accent,
            shape: BoxShape.circle,
            boxShadow: GlassShadow.glow(accent),
          ),
          child: const Icon(LucideIcons.plus, size: 28, color: Color(0xFF000000)),
        ),
      ),
    );
  }
}
