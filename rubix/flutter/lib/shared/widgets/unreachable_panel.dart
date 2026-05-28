import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// "Agent offline" empty state — surfaced when the rubix-agent can't be
/// reached but the rest of the app is healthy. Amber `wifi-off` glyph in
/// a soft rounded square, calm copy, last-seen note, single retry action.
///
/// Per Figma `?node-id=11-4`.
class UnreachablePanel extends StatelessWidget {
  const UnreachablePanel({
    this.onRetry,
    this.lastSeen,
    this.title = 'Agent offline',
    this.message =
        "We can't reach the Rubix agent right now. "
        'Check that the service is running, then try again.',
    super.key,
  });

  final VoidCallback? onRetry;
  final String? lastSeen;
  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 380),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  color: t.warning.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(16),
                  border: Border.all(
                    color: t.warning.withValues(alpha: 0.30),
                  ),
                ),
                alignment: Alignment.center,
                child: Icon(LucideIcons.wifiOff, size: 26, color: t.warning),
              ),
              const SizedBox(height: 16),
              Text(
                title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 16,
                  fontWeight: FontWeight.w600,
                  letterSpacing: -0.2,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                message,
                textAlign: TextAlign.center,
                style: TextStyle(color: t.muted, fontSize: 13, height: 1.45),
              ),
              if (lastSeen != null) ...[
                const SizedBox(height: 10),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 5,
                  ),
                  decoration: BoxDecoration(
                    color: t.surface2,
                    borderRadius: BorderRadius.circular(999),
                    border: Border.all(color: t.border),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(LucideIcons.clock, size: 11, color: t.muted),
                      const SizedBox(width: 6),
                      Text(
                        'Last seen $lastSeen',
                        style: TextStyle(
                          color: t.muted,
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                          letterSpacing: 0.1,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
              if (onRetry != null) ...[
                const SizedBox(height: 18),
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
      ),
    );
  }
}
