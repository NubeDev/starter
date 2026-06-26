import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// Visual variants for [GhostButton] — the DNA kit's three secondary actions
/// (kit frame 6:28, Device-detail 7:35):
///   • [teal]    — "Place on page": teal icon + label, teal-tinted hairline.
///   • [neutral] — "Label": ink icon + label, neutral glass hairline.
///   • [danger]  — "Decommission": red icon + label, red-tinted hairline.
enum GhostButtonVariant { teal, neutral, danger }

/// A compact, transparent-fill button with an icon + label and a 1px hairline
/// border — the DNA "ghost" action. Springs on tap via [Pressable]. For the
/// full-width accent CTA use `PrimaryButton` instead.
class GhostButton extends StatelessWidget {
  const GhostButton({
    required this.label,
    required this.icon,
    this.onPressed,
    this.variant = GhostButtonVariant.neutral,
    this.expand = false,
    super.key,
  });

  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
  final GhostButtonVariant variant;

  /// When true the button fills its parent's width with centered content —
  /// for the Device-detail action row where three sit in equal [Expanded]s.
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final disabled = onPressed == null;

    // Resolve the foreground tint + border per variant. Danger uses the fixed
    // status red; teal tracks the live accent; neutral is plain ink.
    final (Color fg, Color border) = switch (variant) {
      GhostButtonVariant.teal => (
          look.accent,
          look.accent.withValues(alpha: 0.45),
        ),
      GhostButtonVariant.neutral => (look.ink, Glass.border),
      GhostButtonVariant.danger => (
          RubixTokens.fault,
          RubixTokens.fault.withValues(alpha: 0.45),
        ),
    };

    return Pressable(
      onTap: disabled ? null : onPressed,
      semanticLabel: label,
      child: Opacity(
        opacity: disabled ? 0.4 : 1,
        child: Container(
          width: expand ? double.infinity : null,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 11),
          decoration: BoxDecoration(
            color: Glass.fill,
            borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            border: Border.all(color: border),
          ),
          child: Row(
            mainAxisSize: expand ? MainAxisSize.max : MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(icon, size: 14, color: fg),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: fg,
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
