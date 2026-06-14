import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// A pill toggle chip — ported from the React `Chip`. Filled with the primary
/// accent when [active], otherwise a faint translucent surface.
class GlassChip extends StatelessWidget {
  const GlassChip({
    required this.label,
    this.active = false,
    this.onTap,
    super.key,
  });

  final String label;
  final bool active;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Pressable(
      onTap: onTap,
      scale: 0.94,
      semanticLabel: label,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        decoration: BoxDecoration(
          color: active ? RubixTokens.primary : const Color(0x0FFFFFFF),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: active ? RubixTokens.primaryOn : RubixTokens.inkVariant,
            fontSize: 14,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
    );
  }
}
