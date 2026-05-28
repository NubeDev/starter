import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/demo/demo_data.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';
import 'package:rubix_flutter/shared/widgets/scaffold/ambient_glow_background.dart';

/// Connections — Figma node 6-79. Header glass pill ("NETWORK"),
/// "Connected devices." serif accent, search bar, then a list of
/// connected-device rows pulled from `kDemoConnectedDevices`.
class ConnectionsListScreen extends ConsumerStatefulWidget {
  const ConnectionsListScreen({super.key});

  @override
  ConsumerState<ConnectionsListScreen> createState() =>
      _ConnectionsListScreenState();
}

class _ConnectionsListScreenState
    extends ConsumerState<ConnectionsListScreen> {
  final _searchCtl = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _searchCtl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;

    final devices = _filter(kDemoConnectedDevices, _query);
    final connected = kDemoConnectedDevices
        .where((d) => d.status == DemoConnStatus.connected)
        .length;
    final attention = kDemoConnectedDevices
        .where((d) => d.status != DemoConnStatus.connected)
        .length;

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
            children: [
              const SizedBox(height: 6),
              const _NetworkGlassPill(),
              const SizedBox(height: 18),
              const _Hero(),
              const SizedBox(height: 10),
              Text(
                '$connected active · $attention needs attention',
                style: TextStyle(color: t.muted, fontSize: 13, height: 1.45),
              ),
              const SizedBox(height: 20),
              _SearchField(
                controller: _searchCtl,
                onChanged: (v) => setState(() => _query = v),
              ),
              const SizedBox(height: 18),
              if (devices.isEmpty)
                _EmptyState(query: _query)
              else
                Column(
                  children: [
                    for (final d in devices) ...[
                      _DeviceRow(device: d),
                      const SizedBox(height: 12),
                    ],
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }

  List<DemoConnectedDevice> _filter(
    List<DemoConnectedDevice> all,
    String q,
  ) {
    if (q.trim().isEmpty) return all;
    final needle = q.toLowerCase();
    return all
        .where(
          (d) =>
              d.name.toLowerCase().contains(needle) ||
              d.type.toLowerCase().contains(needle),
        )
        .toList(growable: false);
  }
}

// ────────────────────────────────────────────────────────────────────────
// Network glass pill — node 6:115–6:120.
// ────────────────────────────────────────────────────────────────────────

class _NetworkGlassPill extends StatelessWidget {
  const _NetworkGlassPill();

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    const dot = Color(0xFF21C45D);
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: t.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(
                color: dot,
                shape: BoxShape.circle,
                boxShadow: [
                  BoxShadow(
                    color: dot.withValues(alpha: 0.55),
                    blurRadius: 6,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Text(
              'NETWORK',
              style: TextStyle(
                color: t.text,
                fontSize: 11,
                fontWeight: FontWeight.w600,
                letterSpacing: 2.0,
              ),
            ),
          ],
        ),
      ),
    );
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
// Search field — node 6:121–6:125. Single full-width bar, no add button.
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
                hintText: 'Search devices, agents, sites',
                hintStyle: TextStyle(color: t.muted, fontSize: 13.5),
                filled: false,
                border: InputBorder.none,
                enabledBorder: InputBorder.none,
                focusedBorder: InputBorder.none,
                disabledBorder: InputBorder.none,
                errorBorder: InputBorder.none,
                focusedErrorBorder: InputBorder.none,
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

// ────────────────────────────────────────────────────────────────────────
// Device row — node 6:126–6:168. ~72px tall solid card.
// ────────────────────────────────────────────────────────────────────────

class _DeviceRow extends StatelessWidget {
  const _DeviceRow({required this.device});
  final DemoConnectedDevice device;

  ({Color color, String label}) _statusInfo(BuildContext context) {
    final t = Theme.of(context).nube;
    switch (device.status) {
      case DemoConnStatus.connected:
        return (color: t.success, label: 'CONNECTED');
      case DemoConnStatus.warning:
        return (color: t.warning, label: 'WARNING');
      case DemoConnStatus.offline:
        return (color: t.danger, label: 'OFFLINE');
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final status = _statusInfo(context);

    return SizedBox(
      height: 72,
      child: NubeGlowCard(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Row(
          children: [
            // Teal-tinted icon tile.
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: t.leaf.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(10),
              ),
              alignment: Alignment.center,
              child: Icon(device.icon, size: 20, color: t.leaf),
            ),
            const SizedBox(width: 14),
            // Title + meta.
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Text(
                    device.name,
                    style: TextStyle(
                      color: t.text,
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      height: 1.2,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 3),
                  Text(
                    '${device.type} · ${device.detail}',
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
            const SizedBox(width: 10),
            // Status text + dot, with chevron beneath.
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              mainAxisAlignment: MainAxisAlignment.center,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      status.label,
                      style: TextStyle(
                        color: status.color,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        letterSpacing: 0.6,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      width: 8,
                      height: 8,
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
                  ],
                ),
                const SizedBox(height: 6),
                Icon(
                  LucideIcons.chevronRight,
                  size: 16,
                  color: t.muted,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Empty state — search returned nothing.
// ────────────────────────────────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.query});
  final String query;

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
            child: Icon(LucideIcons.searchX, size: 22, color: t.muted),
          ),
          const SizedBox(height: 14),
          Text(
            'No devices match "$query".',
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
