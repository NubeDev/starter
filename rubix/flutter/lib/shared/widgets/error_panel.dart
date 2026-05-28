import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Visual intent for [ErrorPanel].
///
/// [empty] uses calm, muted surfaces — for "no data", "could not load",
/// or unauthenticated states. [destructive] uses red — reserved for hard
/// failures the user should treat as a fault.
enum ErrorPanelIntent { empty, destructive }

/// Displays an error or empty-state message with an optional retry action.
///
/// Defaults to the on-brand muted treatment (no red fills) — per
/// DESIGN.md §9 "Style the empty/error state on-brand: muted icon in a
/// soft rounded square, quiet heading, one clear action."
class ErrorPanel extends StatelessWidget {
  const ErrorPanel({
    required this.message,
    this.onRetry,
    this.intent = ErrorPanelIntent.empty,
    this.icon = LucideIcons.alertCircle,
    super.key,
  });

  final String message;
  final VoidCallback? onRetry;
  final ErrorPanelIntent intent;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final destructive = intent == ErrorPanelIntent.destructive;
    final bg = destructive ? t.danger.withValues(alpha: 0.10) : t.surface2;
    final border = destructive ? t.danger.withValues(alpha: 0.25) : t.border;
    final iconColor = destructive ? t.danger : t.muted;

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 44,
              height: 44,
              decoration: BoxDecoration(
                color: bg,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: border),
              ),
              alignment: Alignment.center,
              child: Icon(icon, size: 20, color: iconColor),
            ),
            const SizedBox(height: 14),
            Text(
              message,
              textAlign: TextAlign.center,
              style: TextStyle(
                color: t.text,
                fontSize: 14,
                fontWeight: FontWeight.w500,
              ),
            ),
            if (onRetry != null) ...[
              const SizedBox(height: 14),
              NubeButton(
                label: 'Retry',
                icon: LucideIcons.refreshCw,
                variant: NubeButtonVariant.secondary,
                size: NubeButtonSize.sm,
                onPressed: onRetry,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
