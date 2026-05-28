import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Dark, hairline-bordered surface with an optional radial corner glow.
///
/// The signature "shadcn dashboard" tile: subtle warmth from a single
/// corner tint, no fills, no elevation. Used for KPI tiles, the chart
/// panes, the activity feed, and the settings sections.
class NubeGlowCard extends StatelessWidget {
  const NubeGlowCard({
    required this.child,
    this.tone = NubeGlowTone.none,
    this.padding = const EdgeInsets.all(20),
    this.borderRadius = 14,
    this.onTap,
    super.key,
  });

  final Widget child;
  final NubeGlowTone tone;
  final EdgeInsetsGeometry padding;
  final double borderRadius;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final radius = BorderRadius.circular(borderRadius);
    final glow = t.glowFor(tone);

    final card = DecoratedBox(
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: radius,
        border: Border.all(color: t.border),
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: Stack(
          children: [
            if (tone != NubeGlowTone.none)
              Positioned.fill(
                child: IgnorePointer(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      gradient: RadialGradient(
                        center: Alignment.topRight,
                        radius: 1.1,
                        colors: [glow, const Color(0x00000000)],
                        stops: const [0, 0.7],
                      ),
                    ),
                  ),
                ),
              ),
            Padding(padding: padding, child: child),
          ],
        ),
      ),
    );

    if (onTap == null) return card;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: card,
      ),
    );
  }
}
