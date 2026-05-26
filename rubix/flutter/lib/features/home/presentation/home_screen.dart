import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';
import 'package:rubix_flutter/shared/widgets/unreachable_panel.dart';

/// Pinned home screen — chassis validation for Block 5.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final activeAsync = ref.watch(activeConnectionProvider);
    final healthAsync = ref.watch(agentHealthProvider);
    final userAsync = ref.watch(currentUserProvider);

    return RefreshIndicator(
      onRefresh: () async {
        ref
          ..invalidate(agentHealthProvider)
          ..invalidate(currentUserProvider);
        await Future.wait<void>([
          ref.read(agentHealthProvider.future),
          ref.read(currentUserProvider.future).catchError((_) {
            throw Exception('ignored');
          }),
        ]);
      },
      child: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          _SectionHeader(label: l.agentHealthSection),
          const SizedBox(height: 10),
          healthAsync.when(
            loading: () => const _PillSkeleton(),
            error: (e, _) => _HealthPill.unreachable(label: l.agentUnreachable),
            data: (health) => switch (health) {
              AgentHealthOk() => _HealthPill.ok(label: l.agentHealthy),
              AgentHealthBadStatus(:final statusCode) =>
                _HealthPill.unreachable(
                  label: '${l.agentUnreachable} ($statusCode)',
                ),
              AgentHealthUnreachable() =>
                _HealthPill.unreachable(label: l.agentUnreachable),
            },
          ),
          const SizedBox(height: 28),

          _SectionHeader(label: l.currentUserSection),
          const SizedBox(height: 10),
          userAsync.when(
            loading: () => const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: LoadingIndicator(),
            ),
            error: (e, _) => ErrorPanel(
              message: l.currentUserError,
              onRetry: () => ref.invalidate(currentUserProvider),
            ),
            data: (user) => NubeCard(
              child: _InfoRow(
                icon: LucideIcons.user,
                title: user.email,
                subtitle: user.role,
              ),
            ),
          ),
          const SizedBox(height: 28),

          _SectionHeader(label: l.activeConnectionSection),
          const SizedBox(height: 10),
          activeAsync.when(
            loading: () => const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: LoadingIndicator(),
            ),
            error: (e, _) => ErrorPanel(message: e.toString()),
            data: (conn) {
              if (conn == null) {
                return UnreachablePanel(
                  onRetry: () => context.push('/connections'),
                );
              }
              return NubeCard(
                onTap: () => context.push('/connections'),
                child: _InfoRow(
                  icon: LucideIcons.link,
                  title: conn.label,
                  subtitle: conn.baseUrl,
                  trailing: Icon(
                    LucideIcons.settings2,
                    size: 16,
                    color: Theme.of(context).nube.muted,
                  ),
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Text(
      label.toUpperCase(),
      style: TextStyle(
        color: t.muted,
        fontSize: 11,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.6,
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Row(
      children: [
        Container(
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: t.surface2,
            borderRadius: BorderRadius.circular(8),
          ),
          alignment: Alignment.center,
          child: Icon(icon, size: 16, color: t.muted),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  height: 1.25,
                ),
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: TextStyle(
                  color: t.muted,
                  fontSize: 12.5,
                  height: 1.3,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
        if (trailing != null) ...[
          const SizedBox(width: 8),
          trailing!,
        ],
      ],
    );
  }
}

class _HealthPill extends StatelessWidget {
  const _HealthPill._({
    required this.label,
    required this.color,
    required this.icon,
  });

  factory _HealthPill.ok({required String label}) => _HealthPill._(
        label: label,
        color: const Color(0xFF21C45D),
        icon: LucideIcons.checkCircle,
      );

  factory _HealthPill.unreachable({required String label}) => _HealthPill._(
        label: label,
        color: const Color(0xFFEF4343),
        icon: LucideIcons.alertCircle,
      );

  final String label;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withValues(alpha: 0.30)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: 12.5,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

class _PillSkeleton extends StatelessWidget {
  const _PillSkeleton();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 120,
      height: 26,
      decoration: BoxDecoration(
        color: Theme.of(context).nube.surface2,
        borderRadius: BorderRadius.circular(6),
      ),
    );
  }
}
