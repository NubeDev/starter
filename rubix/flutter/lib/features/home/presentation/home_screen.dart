import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/unreachable_panel.dart';

/// Pinned home screen — chassis validation for Block 5.
///
/// Renders:
///   - Agent `/healthz` status pill (green / red).
///   - Current user (email + role) from `/api/v1/auth/me`.
///   - Active connection (label + baseUrl) from the local store.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final activeAsync = ref.watch(activeConnectionProvider);
    final healthAsync = ref.watch(agentHealthProvider);
    final userAsync = ref.watch(currentUserProvider);

    return Scaffold(
      body: RefreshIndicator(
        onRefresh: () async {
          ref
            ..invalidate(agentHealthProvider)
            ..invalidate(currentUserProvider);
          await Future.wait<void>([
            ref.read(agentHealthProvider.future),
            ref.read(currentUserProvider.future).catchError((_) {
              // Surface via the AsyncValue branch, not here.
              throw Exception('ignored');
            }),
          ]);
        },
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            // --- Agent health pill --------------------------------------
            _SectionHeader(label: l.agentHealthSection),
            const SizedBox(height: 8),
            healthAsync.when(
              loading: () => const _PillSkeleton(),
              error: (e, _) =>
                  _HealthPill.unreachable(label: l.agentUnreachable),
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

            const SizedBox(height: 24),

            // --- Current user -------------------------------------------
            _SectionHeader(label: l.currentUserSection),
            const SizedBox(height: 8),
            userAsync.when(
              loading: () => const Padding(
                padding: EdgeInsets.symmetric(vertical: 24),
                child: LoadingIndicator(),
              ),
              error: (e, _) => ErrorPanel(
                message: l.currentUserError,
                onRetry: () => ref.invalidate(currentUserProvider),
              ),
              data: (user) => Card(
                child: ListTile(
                  leading: const Icon(Icons.person_outline),
                  title: Text(user.email),
                  subtitle: Text(user.role),
                ),
              ),
            ),

            const SizedBox(height: 24),

            // --- Active connection --------------------------------------
            _SectionHeader(label: l.activeConnectionSection),
            const SizedBox(height: 8),
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
                return Card(
                  child: ListTile(
                    leading: const Icon(Icons.link),
                    title: Text(conn.label),
                    subtitle: Text(conn.baseUrl),
                    trailing: IconButton(
                      icon: const Icon(Icons.tune),
                      tooltip: l.manageConnections,
                      onPressed: () => context.push('/connections'),
                    ),
                  ),
                );
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    return Text(
      label,
      style: Theme.of(context).textTheme.titleSmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
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
        color: Colors.green,
        icon: Icons.check_circle,
      );

  factory _HealthPill.unreachable({required String label}) => _HealthPill._(
        label: label,
        color: Colors.red,
        icon: Icons.error,
      );

  final String label;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        // ignore: deprecated_member_use
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, color: color, size: 18),
          const SizedBox(width: 8),
          Text(label, style: TextStyle(color: color)),
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
      height: 32,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
    );
  }
}
