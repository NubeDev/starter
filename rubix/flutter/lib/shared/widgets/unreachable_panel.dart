import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Displays a panel indicating the server is unreachable.
class UnreachablePanel extends StatelessWidget {
  const UnreachablePanel({this.onRetry, super.key});

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
                color: t.surface2,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: t.border),
              ),
              alignment: Alignment.center,
              child: Icon(LucideIcons.cloudOff, size: 22, color: t.muted),
            ),
            const SizedBox(height: 14),
            Text(
              'Server unreachable',
              style: TextStyle(
                color: t.text,
                fontSize: 15,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              'Could not reach the server. Check your connection and try again.',
              textAlign: TextAlign.center,
              style: TextStyle(color: t.muted, fontSize: 13),
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
