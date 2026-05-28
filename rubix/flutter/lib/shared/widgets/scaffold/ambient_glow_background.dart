import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Ambient teal-glow background used behind every primary screen.
///
/// Composes three soft radial blobs of brand teal — per Figma node `2-8`,
/// children `2:9` (top-right, large), `2:11` (mid-right, medium), and
/// `2:10` (bottom-left, large). Each blob is positioned partially
/// off-screen so its bloom bleeds in past the viewport edge.
///
/// The legacy parameters [glowAlignment], [glowColor], [glowRadius] are
/// retained for source compatibility but ignored — the composition is
/// fixed to match the design system.
class AmbientGlowBackground extends StatelessWidget {
  const AmbientGlowBackground({
    super.key,
    required this.child,
    @Deprecated('Layout is fixed to the Figma composition.')
    this.glowAlignment = Alignment.topRight,
    @Deprecated('Always teal — pulled from NubeTokens.leaf.')
    this.glowColor,
    @Deprecated('Radii are fixed to match the Figma composition.')
    this.glowRadius = 0.65,
  });

  final Widget child;
  final Alignment glowAlignment;
  final Color? glowColor;
  final double glowRadius;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final teal = t.leaf;

    // Dark mode pushes a more visible bloom; light mode stays gentle.
    final aTop = isDark ? 0.32 : 0.18;
    final aMid = isDark ? 0.22 : 0.12;
    final aBot = isDark ? 0.28 : 0.16;

    return Container(
      color: t.bg,
      child: Stack(
        fit: StackFit.expand,
        children: [
          IgnorePointer(
            child: Stack(
              children: [
                // Blob A — node 2:9 — large top-right, off the top edge.
                Positioned(
                  top: -140,
                  right: -120,
                  child: _GlowBlob(
                    size: 360,
                    color: teal.withValues(alpha: aTop),
                  ),
                ),
                // Blob B — node 2:11 — medium, mid-right.
                Positioned(
                  top: 0,
                  bottom: 0,
                  right: -80,
                  child: Center(
                    child: _GlowBlob(
                      size: 260,
                      color: teal.withValues(alpha: aMid),
                    ),
                  ),
                ),
                // Blob C — node 2:10 — large bottom-left, off the left edge.
                Positioned(
                  bottom: -120,
                  left: -140,
                  child: _GlowBlob(
                    size: 340,
                    color: teal.withValues(alpha: aBot),
                  ),
                ),
              ],
            ),
          ),
          Positioned.fill(child: child),
        ],
      ),
    );
  }
}

/// A single circular teal bloom — built from a radial gradient that fades
/// to transparent at the disc's edge. Soft enough to read as a halo
/// without a hard outline.
class _GlowBlob extends StatelessWidget {
  const _GlowBlob({required this.size, required this.color});

  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: DecoratedBox(
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          gradient: RadialGradient(
            colors: [color, color.withValues(alpha: 0), Colors.transparent],
            stops: const [0, 0.7, 1],
          ),
        ),
      ),
    );
  }
}

/// Corner halo used behind a single hero card — per Figma node `2:29`,
/// the DEVICES tile gets a soft teal radial at its top-right corner.
///
/// Wrap a `NubeGlowCard` (or any other tile) in a `Stack` with
/// `clipBehavior: Clip.none` and add this as the first child positioned
/// off the card's top-right corner.
class CornerHalo extends StatelessWidget {
  const CornerHalo({
    super.key,
    this.size = 120,
    this.alignment = Alignment.topRight,
  });

  final double size;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final color = t.leaf.withValues(alpha: isDark ? 0.35 : 0.20);
    return IgnorePointer(
      child: SizedBox(
        width: size,
        height: size,
        child: DecoratedBox(
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            gradient: RadialGradient(
              colors: [color, color.withValues(alpha: 0), Colors.transparent],
              stops: const [0, 0.7, 1],
            ),
          ),
        ),
      ),
    );
  }
}
