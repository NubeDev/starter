import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/connections/presentation/edit_connection/edit_connection_screen.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

class ConnectionsListScreen extends ConsumerWidget {
  const ConnectionsListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final listAsync = ref.watch(connectionListControllerProvider);

    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 20, 20, 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  l.connections,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 20,
                    fontWeight: FontWeight.w700,
                    letterSpacing: -0.3,
                  ),
                ),
              ),
              NubeButton(
                label: 'Add',
                icon: LucideIcons.plus,
                size: NubeButtonSize.sm,
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => const AddConnectionScreen(),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Expanded(
            child: listAsync.when(
              loading: () => const LoadingIndicator(),
              error: (e, _) => Center(
                child: Text(
                  '${l.error}: $e',
                  style: TextStyle(color: t.danger, fontSize: 13),
                ),
              ),
              data: (connections) => connections.isEmpty
                  ? _EmptyState(label: l.noConnections)
                  : ListView.separated(
                      itemCount: connections.length,
                      separatorBuilder: (_, __) => const SizedBox(height: 8),
                      itemBuilder: (context, index) =>
                          _ConnectionRow(connection: connections[index]),
                    ),
            ),
          ),
        ],
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Center(
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
            child: Icon(LucideIcons.unlink, size: 22, color: t.muted),
          ),
          const SizedBox(height: 14),
          Text(
            label,
            style: TextStyle(
              color: t.muted,
              fontSize: 13.5,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}

class _ConnectionRow extends ConsumerWidget {
  const _ConnectionRow({required this.connection});

  final Connection connection;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final t = Theme.of(context).nube;
    final activeAsync = ref.watch(activeConnectionProvider);
    final isActive = activeAsync.value?.id == connection.id;

    return Dismissible(
      key: ValueKey(connection.id),
      direction: DismissDirection.endToStart,
      background: Container(
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        decoration: BoxDecoration(
          color: t.danger.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: t.danger.withValues(alpha: 0.3)),
        ),
        child: Icon(LucideIcons.trash2, size: 18, color: t.danger),
      ),
      onDismissed: (_) => ref
          .read(connectionListControllerProvider.notifier)
          .delete(connection.id),
      child: NubeCard(
        onTap: () async {
          final messenger = ScaffoldMessenger.of(context);
          try {
            await ref
                .read(connectionListControllerProvider.notifier)
                .activate(connection.id);
          } catch (e) {
            messenger.showSnackBar(
              SnackBar(content: Text('Sign-in failed: $e')),
            );
          }
        },
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: isActive
                    ? t.leaf.withValues(alpha: 0.12)
                    : t.surface2,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: isActive
                      ? t.leaf.withValues(alpha: 0.3)
                      : Colors.transparent,
                ),
              ),
              alignment: Alignment.center,
              child: Icon(
                isActive ? LucideIcons.checkCircle2 : LucideIcons.server,
                size: 15,
                color: isActive ? t.leaf : t.muted,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Flexible(
                        child: Text(
                          connection.label,
                          style: TextStyle(
                            color: t.text,
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            height: 1.25,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (isActive) ...[
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: t.leaf.withValues(alpha: 0.12),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            'Active',
                            style: TextStyle(
                              color: t.leaf,
                              fontSize: 10,
                              fontWeight: FontWeight.w700,
                              letterSpacing: 0.4,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    connection.baseUrl,
                    style: TextStyle(
                      color: t.muted,
                      fontSize: 12,
                      height: 1.3,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            _RowIconButton(
              icon: LucideIcons.pencil,
              tooltip: 'Edit',
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) =>
                      EditConnectionScreen(connection: connection),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RowIconButton extends StatefulWidget {
  const _RowIconButton({
    required this.icon,
    required this.onTap,
    this.tooltip,
  });
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;

  @override
  State<_RowIconButton> createState() => _RowIconButtonState();
}

class _RowIconButtonState extends State<_RowIconButton> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final btn = MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          width: 32,
          height: 32,
          decoration: BoxDecoration(
            color: _hover ? t.surface2 : Colors.transparent,
            borderRadius: BorderRadius.circular(6),
          ),
          alignment: Alignment.center,
          child: Icon(widget.icon, size: 15, color: t.muted),
        ),
      ),
    );
    return widget.tooltip != null
        ? Tooltip(message: widget.tooltip!, child: btn)
        : btn;
  }
}
