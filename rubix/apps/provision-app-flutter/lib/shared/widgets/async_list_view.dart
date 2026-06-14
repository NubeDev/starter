import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// The four states a fetched list can be in. Replaces the old "swallow the
/// error, show an empty box" pattern so a failed query (401, 500, tenant
/// mismatch, unreachable agent) shows a real message + retry instead of looking
/// identical to "no data".
enum ListPhase { loading, error, empty, data }

/// Renders the non-data states (loading / error / empty) as glass panels, so
/// every list screen reports failures the same way. When [phase] is
/// [ListPhase.data] this returns the [child].
class AsyncListView extends StatelessWidget {
  const AsyncListView({
    required this.phase,
    required this.child,
    this.error,
    this.emptyText = 'Nothing here yet.',
    this.emptyIcon = LucideIcons.inbox,
    this.onRetry,
    super.key,
  });

  final ListPhase phase;
  final Widget child;
  final String? error;
  final String emptyText;
  final IconData emptyIcon;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    switch (phase) {
      case ListPhase.data:
        return child;
      case ListPhase.loading:
        return const Padding(
          padding: EdgeInsets.symmetric(vertical: 48),
          child: Center(
            child: SizedBox(
              width: 22,
              height: 22,
              child: CircularProgressIndicator(
                strokeWidth: 2.4,
                color: RubixTokens.primary,
              ),
            ),
          ),
        );
      case ListPhase.empty:
        return _Panel(
          icon: emptyIcon,
          color: RubixTokens.inkMuted,
          text: emptyText,
        );
      case ListPhase.error:
        return _Panel(
          icon: LucideIcons.alertTriangle,
          color: RubixTokens.fault,
          text: error ?? 'Could not load.',
          onRetry: onRetry,
        );
    }
  }
}

class _Panel extends StatelessWidget {
  const _Panel({
    required this.icon,
    required this.color,
    required this.text,
    this.onRetry,
  });

  final IconData icon;
  final Color color;
  final String text;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: GlassSurface(
        borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 40),
        child: Column(
          children: [
            Icon(icon, size: 32, color: color),
            const SizedBox(height: 12),
            Text(
              text,
              textAlign: TextAlign.center,
              style: const TextStyle(color: RubixTokens.inkVariant, fontSize: 14),
            ),
            if (onRetry != null) ...[
              const SizedBox(height: 16),
              Pressable(
                onTap: onRetry,
                semanticLabel: 'Retry',
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
                    border:
                        Border.all(color: Colors.white.withValues(alpha: 0.15)),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(LucideIcons.refreshCw,
                          size: 16, color: RubixTokens.ink),
                      SizedBox(width: 8),
                      Text(
                        'Retry',
                        style: TextStyle(
                          color: RubixTokens.ink,
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
