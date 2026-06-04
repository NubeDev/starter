import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/features/devices/domain/status_dot.dart';
import 'package:provision_app/features/devices/presentation/device_detail_screen.dart';
import 'package:provision_app/features/devices/presentation/place_on_page_sheet.dart';
import 'package:provision_app/shared/widgets/async_list_view.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// Device list with search; tap a row to drill into points + per-point toggles.
/// Ported from the React `Devices`. Filters client-side by id/name/template and
/// re-fetches whenever the shared refresh signal bumps.
class DevicesScreen extends ConsumerStatefulWidget {
  const DevicesScreen({super.key});

  @override
  ConsumerState<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends ConsumerState<DevicesScreen> {
  final _search = TextEditingController();
  List<DeviceRow> _rows = const [];
  String _q = '';
  int _loadedFor = -1;
  ListPhase _phase = ListPhase.loading;
  String? _error;

  Future<void> _load() async {
    setState(() {
      _phase = ListPhase.loading;
      _error = null;
    });
    try {
      final list = await ref.read(bcApiProvider).devicesList();
      if (!mounted) return;
      setState(() {
        _rows = list;
        _phase = list.isEmpty ? ListPhase.empty : ListPhase.data;
      });
    } catch (e) {
      if (!mounted) return;
      // No longer swallowed — a 401/500/unreachable agent now shows a real
      // message + retry instead of an indistinguishable "empty" list.
      setState(() {
        _phase = ListPhase.error;
        _error = e.toString();
      });
    }
  }

  Future<void> _reload() {
    _loadedFor = ref.read(refreshProvider);
    return _load();
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  List<DeviceRow> get _filtered {
    final t = _q.trim().toLowerCase();
    if (t.isEmpty) return _rows;
    return _rows
        .where(
          (r) =>
              r.deviceId.toLowerCase().contains(t) ||
              (r.name ?? '').toLowerCase().contains(t) ||
              r.template.toLowerCase().contains(t),
        )
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    final look = context.look;
    final filtered = _filtered;

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const PageHeader(eyebrow: 'Inventory', title: 'Devices'),
          GlassSurface(
            borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: [
                Icon(LucideIcons.search,
                    size: 16, color: look.inkMuted),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _search,
                    onChanged: (v) => setState(() => _q = v),
                    style: TextStyle(
                        color: look.ink, fontSize: 16),
                    cursorColor: look.accent,
                    decoration: InputDecoration(
                      isDense: true,
                      hintText: 'Search devices',
                      hintStyle: TextStyle(color: look.inkMuted),
                      border: InputBorder.none,
                      contentPadding:
                          const EdgeInsets.symmetric(vertical: 12),
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),
          AsyncListView(
            phase: _phase,
            error: _error,
            emptyText: 'No devices yet. Scan one to get started.',
            onRetry: _reload,
            child: Column(
              children: [
                for (final d in filtered) ...[
                  _DeviceCard(device: d),
                  const SizedBox(height: 8),
                ],
                // All loaded but the search filtered everything out.
                if (filtered.isEmpty && _rows.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 24),
                    child: Text(
                      'No matches.',
                      style: TextStyle(
                          color: look.inkMuted, fontSize: 14),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _DeviceCard extends ConsumerWidget {
  const _DeviceCard({required this.device});
  final DeviceRow device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = context.look;
    return GlassCard(
      onTap: () => Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (_) => DeviceDetailScreen(device: device),
        ),
      ),
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: const Color(0x0FFFFFFF),
              borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            ),
            child: Icon(LucideIcons.cpu,
                size: 20, color: look.inkSoft),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  device.name ?? device.deviceId,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: look.ink,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  device.network != null && device.network!.isNotEmpty
                      ? '${device.template} · ${device.network}'
                      : device.template,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                      color: look.inkMuted, fontSize: 12),
                ),
              ],
            ),
          ),
          if (isPlaceable(device)) ...[
            const SizedBox(width: 8),
            _PlacePill(device: device),
          ],
          const SizedBox(width: 8),
          Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(
              color: statusColor(device.status, look),
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 8),
          Icon(LucideIcons.chevronRight,
              size: 20, color: look.inkMuted),
        ],
      ),
    );
  }
}

class _PlacePill extends ConsumerWidget {
  const _PlacePill({required this.device});
  final DeviceRow device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = context.look;
    final pending = statusColor('pending', look);
    return Pressable(
      scale: 0.94,
      semanticLabel: 'Place on page',
      onTap: () => showPlaceOnPageSheet(context, ref, device),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: pending.withValues(alpha: 0.133),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(LucideIcons.mapPin, size: 14, color: pending),
            const SizedBox(width: 4),
            Text(
              'Place',
              style: TextStyle(
                color: pending,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

