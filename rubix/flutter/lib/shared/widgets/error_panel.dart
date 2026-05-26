import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Displays an error message with an optional retry action.
class ErrorPanel extends StatelessWidget {
  const ErrorPanel({required this.message, this.onRetry, super.key});

  final String message;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                color: t.danger.withValues(alpha: 0.10),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: t.danger.withValues(alpha: 0.25)),
              ),
              alignment: Alignment.center,
              child: Icon(LucideIcons.alertCircle, size: 22, color: t.danger),
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
