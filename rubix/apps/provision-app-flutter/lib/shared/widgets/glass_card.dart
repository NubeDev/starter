import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// A pressable glass card with a spring-y tap — the workhorse surface, ported
/// from the React `GlassCard`. Used everywhere a tappable or static frosted
/// panel is needed. Pass [glowShadow] to override the default soft glass
/// shadow with a colored glow.
class GlassCard extends StatelessWidget {
  const GlassCard({
    required this.child,
    this.onTap,
    this.padding = const EdgeInsets.all(16),
    this.borderRadius =
        const BorderRadius.all(Radius.circular(RubixTokens.radiusMd)),
    this.glowShadow,
    super.key,
  });

  final Widget child;
  final VoidCallback? onTap;
  final EdgeInsetsGeometry padding;
  final BorderRadius borderRadius;
  final List<BoxShadow>? glowShadow;

  @override
  Widget build(BuildContext context) {
    final surface = GlassSurface(
      borderRadius: borderRadius,
      padding: padding,
      boxShadow: glowShadow ?? GlassShadow.glass,
      child: child,
    );
    if (onTap == null) return surface;
    return Pressable(onTap: onTap, child: surface);
  }
}
