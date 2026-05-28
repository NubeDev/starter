import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Visual intent for [ErrorPanel].
///
/// [empty]       — no data / empty list. Calm, muted surfaces.
/// [destructive] — a hard failure. Red icon tile.
enum ErrorPanelIntent { empty, destructive }

/// Displays an error or empty state with optional retry + raw details.
///
/// Per `DESIGN.md` §9 — **no raw exception text in the UI.** The [message]
/// shown to the user must be human copy ("Can't reach the Rubix agent
/// right now"). Raw stack traces / DioException strings go in [details]
/// and surface only behind a "View error details" disclosure.
class ErrorPanel extends StatefulWidget {
  const ErrorPanel({
    required this.message,
    this.title,
    this.details,
    this.onRetry,
    this.retryLabel = 'Try again',
    this.intent = ErrorPanelIntent.empty,
    this.icon,
    super.key,
  });

  final String? title;
  final String message;
  final String? details;
  final VoidCallback? onRetry;
  final String retryLabel;
  final ErrorPanelIntent intent;
  final IconData? icon;

  @override
  State<ErrorPanel> createState() => _ErrorPanelState();
}

class _ErrorPanelState extends State<ErrorPanel> {
  bool _showDetails = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final destructive = widget.intent == ErrorPanelIntent.destructive;
    final accent = destructive ? t.danger : t.muted;
    final bg = destructive ? t.danger.withValues(alpha: 0.10) : t.surface2;
    final border = destructive
        ? t.danger.withValues(alpha: 0.30)
        : t.border;
    final icon = widget.icon ??
        (destructive ? LucideIcons.alertOctagon : LucideIcons.alertCircle);
    final title = widget.title ??
        (destructive ? 'Connection failed' : 'Nothing to show');

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 380),
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
                child: Icon(icon, size: 20, color: accent),
              ),
              const SizedBox(height: 14),
              Text(
                title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  letterSpacing: -0.2,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                widget.message,
                textAlign: TextAlign.center,
                style: TextStyle(color: t.muted, fontSize: 13, height: 1.45),
              ),
              if (widget.onRetry != null) ...[
                const SizedBox(height: 16),
                NubeButton(
                  label: widget.retryLabel,
                  icon: LucideIcons.refreshCw,
                  variant: NubeButtonVariant.secondary,
                  size: NubeButtonSize.sm,
                  onPressed: widget.onRetry,
                ),
              ],
              if (widget.details != null && widget.details!.isNotEmpty) ...[
                const SizedBox(height: 12),
                _DetailsToggle(
                  open: _showDetails,
                  onTap: () => setState(() => _showDetails = !_showDetails),
                ),
                AnimatedSize(
                  duration: const Duration(milliseconds: 180),
                  curve: Curves.easeOutCubic,
                  child: _showDetails
                      ? Padding(
                          padding: const EdgeInsets.only(top: 10),
                          child: Container(
                            width: double.infinity,
                            padding: const EdgeInsets.all(10),
                            decoration: BoxDecoration(
                              color: t.surface2,
                              borderRadius: BorderRadius.circular(8),
                              border: Border.all(color: t.border),
                            ),
                            child: SelectableText(
                              widget.details!,
                              style: TextStyle(
                                color: t.muted,
                                fontFamily: 'monospace',
                                fontSize: 11,
                                height: 1.4,
                              ),
                            ),
                          ),
                        )
                      : const SizedBox.shrink(),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _DetailsToggle extends StatelessWidget {
  const _DetailsToggle({required this.open, required this.onTap});
  final bool open;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onTap,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              open ? LucideIcons.chevronDown : LucideIcons.chevronRight,
              size: 12,
              color: t.muted,
            ),
            const SizedBox(width: 4),
            Text(
              open ? 'Hide error details' : 'View error details',
              style: TextStyle(
                color: t.muted,
                fontSize: 12,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
