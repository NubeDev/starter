import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/connections/presentation/edit_connection/edit_connection_screen.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/scaffold/ambient_glow_background.dart';

/// Connections — Figma-aligned: hero, search, accent-tinted device-style
/// rows with status dot.
class ConnectionsListScreen extends ConsumerStatefulWidget {
  const ConnectionsListScreen({super.key});

  @override
  ConsumerState<ConnectionsListScreen> createState() =>
      _ConnectionsListScreenState();
}

class _ConnectionsListScreenState extends ConsumerState<ConnectionsListScreen> {
  final _searchCtl = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _searchCtl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final listAsync = ref.watch(connectionListControllerProvider);
    final activeAsync = ref.watch(activeConnectionProvider);
    final activeId = activeAsync.value?.id;

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
            children: [
              const SizedBox(height: 6),
              const _Hero(),
              const SizedBox(height: 10),
              Text(
                'Sites, hubs, and gateways under your control.',
                style: TextStyle(color: t.muted, fontSize: 14, height: 1.45),
              ),
              const SizedBox(height: 20),

              // Search + Add.
              Row(
                children: [
                  Expanded(child: _SearchField(controller: _searchCtl, onChanged: (v) => setState(() => _query = v))),
                  const SizedBox(width: 10),
                  _AddButton(
                    onTap: () => Navigator.of(context).push(
                      MaterialPageRoute<void>(
                        builder: (_) => const AddConnectionScreen(),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 18),

              listAsync.when(
                loading: () => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 32),
                  child: LoadingIndicator(),
                ),
                error: (e, _) => ErrorPanel(
                  message: 'Could not load connections.',
                  onRetry: () => ref
                      .read(connectionListControllerProvider.notifier)
                      .refresh(),
                ),
                data: (connections) {
                  final filtered = _filter(connections, _query);
                  if (filtered.isEmpty) {
                    return _EmptyState(label: l.noConnections);
                  }
                  return Column(
                    children: [
                      for (final c in filtered) ...[
                        _ConnectionRow(
                          connection: c,
                          isActive: c.id == activeId,
                        ),
                        const SizedBox(height: 10),
                      ],
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  List<Connection> _filter(List<Connection> all, String q) {
    if (q.trim().isEmpty) return all;
    final needle = q.toLowerCase();
    return all
        .where((c) =>
            c.label.toLowerCase().contains(needle) ||
            c.baseUrl.toLowerCase().contains(needle))
        .toList(growable: false);
  }
}

// ────────────────────────────────────────────────────────────────────────
// Hero
// ────────────────────────────────────────────────────────────────────────

class _Hero extends StatelessWidget {
  const _Hero();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final italic = accentItalicTextStyle(context, fontSize: 38);
    return Text.rich(
      TextSpan(
        children: [
          TextSpan(
            text: 'Connected\n',
            style: TextStyle(
              color: t.text,
              fontSize: 38,
              fontWeight: FontWeight.w600,
              height: 1.05,
              letterSpacing: -0.8,
            ),
          ),
          TextSpan(text: 'devices.', style: italic),
        ],
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Search field
// ────────────────────────────────────────────────────────────────────────

class _SearchField extends StatelessWidget {
  const _SearchField({required this.controller, required this.onChanged});
  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      height: 44,
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: t.border),
      ),
      child: Row(
        children: [
          const SizedBox(width: 14),
          Icon(LucideIcons.search, size: 16, color: t.muted),
          const SizedBox(width: 10),
          Expanded(
            child: TextField(
              controller: controller,
              onChanged: onChanged,
              style: TextStyle(color: t.text, fontSize: 13.5),
              decoration: InputDecoration(
                hintText: 'Search devices…',
                hintStyle: TextStyle(color: t.muted, fontSize: 13.5),
                border: InputBorder.none,
                isDense: true,
                contentPadding: EdgeInsets.zero,
              ),
            ),
          ),
          const SizedBox(width: 14),
        ],
      ),
    );
  }
}

class _AddButton extends StatefulWidget {
  const _AddButton({required this.onTap});
  final VoidCallback onTap;
  @override
  State<_AddButton> createState() => _AddButtonState();
}

class _AddButtonState extends State<_AddButton> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          width: 44,
          height: 44,
          decoration: BoxDecoration(
            color: _hover ? t.leaf : t.leaf.withValues(alpha: 0.9),
            borderRadius: BorderRadius.circular(12),
          ),
          alignment: Alignment.center,
          child: const Icon(LucideIcons.plus, size: 18, color: Colors.white),
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Connection row — device-style with icon tile + status dot.
// ────────────────────────────────────────────────────────────────────────

enum _ConnStatus { connected, warning, offline }

class _ConnectionRow extends ConsumerWidget {
  const _ConnectionRow({required this.connection, required this.isActive});

  final Connection connection;
  final bool isActive;

  _ConnStatus _statusFor() {
    if (isActive) return _ConnStatus.connected;
    final lastUsed = connection.lastUsedAt;
    if (lastUsed == null) return _ConnStatus.offline;
    final age = DateTime.now().difference(lastUsed);
    if (age.inDays > 7) return _ConnStatus.offline;
    if (age.inHours > 24) return _ConnStatus.warning;
    return _ConnStatus.warning;
  }

  ({Color color, String label}) _statusInfo(BuildContext context) {
    final t = Theme.of(context).nube;
    switch (_statusFor()) {
      case _ConnStatus.connected:
        return (color: t.success, label: 'Connected');
      case _ConnStatus.warning:
        return (color: t.warning, label: 'Warning');
      case _ConnStatus.offline:
        return (color: t.danger, label: 'Offline');
    }
  }

  String _subtitle() {
    final last = connection.lastUsedAt;
    final base = Uri.tryParse(connection.baseUrl)?.host.isNotEmpty == true
        ? Uri.parse(connection.baseUrl).host
        : connection.baseUrl;
    if (last == null) return 'Gateway · never seen';
    final diff = DateTime.now().difference(last);
    final ago = diff.inMinutes < 1
        ? 'just now'
        : diff.inHours < 1
            ? '${diff.inMinutes}m ago'
            : diff.inDays < 1
                ? '${diff.inHours}h ago'
                : '${diff.inDays}d ago';
    return 'Gateway · seen $ago · $base';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final t = Theme.of(context).nube;
    final status = _statusInfo(context);

    return Dismissible(
      key: ValueKey(connection.id),
      direction: DismissDirection.endToStart,
      background: Container(
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        decoration: BoxDecoration(
          color: t.danger.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(14),
          border: Border.all(color: t.danger.withValues(alpha: 0.3)),
        ),
        child: Icon(LucideIcons.trash2, size: 18, color: t.danger),
      ),
      onDismissed: (_) => ref
          .read(connectionListControllerProvider.notifier)
          .delete(connection.id),
      child: NubeGlowCard(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
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
        child: Row(
          children: [
            // Icon tile — accent-tinted.
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: t.leaf.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(10),
              ),
              alignment: Alignment.center,
              child: Icon(LucideIcons.server, size: 17, color: t.leaf),
            ),
            const SizedBox(width: 12),
            // Title + subtitle.
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    connection.label,
                    style: TextStyle(
                      color: t.text,
                      fontSize: 14.5,
                      fontWeight: FontWeight.w600,
                      height: 1.2,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 3),
                  Text(
                    _subtitle(),
                    style: TextStyle(
                      color: t.muted,
                      fontSize: 11.5,
                      height: 1.35,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 10),
            // Status dot + label.
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        color: status.color,
                        shape: BoxShape.circle,
                        boxShadow: [
                          BoxShadow(
                            color: status.color.withValues(alpha: 0.5),
                            blurRadius: 6,
                            spreadRadius: 0.5,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      status.label,
                      style: TextStyle(
                        color: status.color,
                        fontSize: 11.5,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                GestureDetector(
                  onTap: () => Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (_) =>
                          EditConnectionScreen(connection: connection),
                    ),
                  ),
                  child: Icon(
                    LucideIcons.chevronRight,
                    size: 16,
                    color: t.muted,
                  ),
                ),
              ],
            ),
          ],
        ),
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
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 40),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 56,
            height: 56,
            decoration: BoxDecoration(
              color: t.surface2,
              borderRadius: BorderRadius.circular(14),
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
