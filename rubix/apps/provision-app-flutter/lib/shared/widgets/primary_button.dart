import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// The full-width accent CTA — ported from the React `PrimaryButton`. When
/// [accent] is passed (the resolved look accent) it tints the fill + glow;
/// otherwise it falls back to the static [tone]. Shows a spinner + label while
/// [loading].
class PrimaryButton extends StatelessWidget {
  const PrimaryButton({
    required this.label,
    this.onPressed,
    this.accent,
    this.tone = PrimaryButtonTone.primary,
    this.loading = false,
    this.icon,
    super.key,
  });

  final String label;
  final VoidCallback? onPressed;

  /// Resolved look accent — overrides [tone] when set.
  final Color? accent;
  final PrimaryButtonTone tone;
  final bool loading;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final disabled = onPressed == null || loading;
    final bg = accent ??
        (tone == PrimaryButtonTone.coral
            ? RubixTokens.coral
            : RubixTokens.primary);
    final fg = tone == PrimaryButtonTone.coral && accent == null
        ? RubixTokens.coralOn
        : RubixTokens.primaryOn;

    return Pressable(
      onTap: disabled ? null : onPressed,
      semanticLabel: label,
      child: Opacity(
        opacity: disabled ? 0.4 : 1,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(vertical: 16),
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
            boxShadow: disabled ? null : GlassShadow.glow(bg),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              if (loading) ...[
                SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2.4, color: fg),
                ),
                const SizedBox(width: 10),
              ] else if (icon != null) ...[
                Icon(icon, size: 20, color: fg),
                const SizedBox(width: 8),
              ],
              Text(
                label,
                style: TextStyle(
                  color: fg,
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

enum PrimaryButtonTone { primary, coral }
