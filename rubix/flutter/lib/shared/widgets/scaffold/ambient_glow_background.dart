import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Ambient glow background for Rubix screens.
class AmbientGlowBackground extends StatelessWidget {
  final Widget child;
  final Alignment glowAlignment;
  final Color? glowColor;
  final double glowRadius;

  const AmbientGlowBackground({
    Key? key,
    required this.child,
    this.glowAlignment = Alignment.topRight,
    this.glowColor,
    this.glowRadius = 0.65,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      decoration: BoxDecoration(
        color: t.bg,
      ),
      child: Stack(
        fit: StackFit.expand,
        children: [
          Positioned.fill(
            child: IgnorePointer(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  gradient: RadialGradient(
                    center: glowAlignment,
                    radius: glowRadius,
                    colors: [
                      (glowColor ?? t.glowFor(NubeGlowTone.teal)).withOpacity(0.18),
                      Colors.transparent,
                    ],
                    stops: const [0, 1],
                  ),
                ),
              ),
            ),
          ),
          Positioned.fill(child: child),
        ],
      ),
    );
  }
}
