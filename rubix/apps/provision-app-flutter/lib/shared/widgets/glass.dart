import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';

/// The core glassmorphic surface — the Flutter port of the React `@utility
/// glass` (Level 1) and `glass-strong`. A frosted, backdrop-blurred panel with
/// a 1px translucent hairline border. Every card/sheet/dock in the app is built
/// on this so the blur amount and border are defined once.
class GlassSurface extends StatelessWidget {
  const GlassSurface({
    required this.child,
    this.borderRadius = const BorderRadius.all(Radius.circular(RubixTokens.radiusMd)),
    this.padding,
    this.strong = false,
    this.boxShadow,
    super.key,
  });

  final Widget child;
  final BorderRadius borderRadius;
  final EdgeInsetsGeometry? padding;

  /// Use the `glass-strong` recipe (opaque-ish dark fill, heavier blur) — for
  /// docks, sheets, and toasts that float over content.
  final bool strong;

  final List<BoxShadow>? boxShadow;

  @override
  Widget build(BuildContext context) {
    final fill = strong ? Glass.strongFill : Glass.fill;
    final border = strong ? Glass.strongBorder : Glass.border;
    final blur = strong ? Glass.strongBlur : Glass.blur;

    return DecoratedBox(
      decoration: BoxDecoration(borderRadius: borderRadius, boxShadow: boxShadow),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: blur, sigmaY: blur),
          child: Container(
            padding: padding,
            decoration: BoxDecoration(
              color: fill,
              borderRadius: borderRadius,
              border: Border.all(color: border),
            ),
            child: child,
          ),
        ),
      ),
    );
  }
}

/// Standard glass shadow recipes — ports of the React `--shadow-glass` /
/// `--shadow-glow-primary` / `--shadow-glow-coral` design tokens.
abstract final class GlassShadow {
  static const glass = [
    BoxShadow(
      color: Color(0x99000000), // rgba(0,0,0,0.6)
      blurRadius: 40,
      offset: Offset(0, 8),
      spreadRadius: -12,
    ),
  ];

  /// A colored glow under a pressable surface — `0 8px 32px -8px <accent>`.
  static List<BoxShadow> glow(Color accent, {double alpha = 0.45}) => [
        BoxShadow(
          color: accent.withValues(alpha: alpha),
          blurRadius: 32,
          offset: const Offset(0, 8),
          spreadRadius: -8,
        ),
      ];
}
