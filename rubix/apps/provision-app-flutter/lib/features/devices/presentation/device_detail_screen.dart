import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/devices/domain/status_dot.dart';
import 'package:provision_app/features/devices/presentation/place_on_page_sheet.dart';
import 'package:provision_app/features/scan/presentation/qr_label.dart';
import 'package:provision_app/shared/widgets/bottom_sheet.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/pressable.dart';
import 'package:provision_app/shared/widgets/toast.dart';

/// Drill-in: a device's points with per-point trend/alarm toggles, inline
/// rename, label sheet, and decommission. Ported from the React `DeviceDetail`.
class DeviceDetailScreen extends ConsumerStatefulWidget {
  const DeviceDetailScreen({required this.device, super.key});
  final DeviceRow device;

  @override
  ConsumerState<DeviceDetailScreen> createState() => _DeviceDetailScreenState();
}

class _DeviceDetailScreenState extends ConsumerState<DeviceDetailScreen> {
  late final TextEditingController _name =
      TextEditingController(text: widget.device.name ?? '');
  List<PointRow> _points = const [];
  bool _renaming = false;
  int _loadedFor = -1;

  DeviceRow get _device => widget.device;

  Future<void> _load() async {
    try {
      final list =
          await ref.read(bcApiProvider).pointsByDevice(_device.deviceId);
      if (mounted) setState(() => _points = list);
    } catch (_) {/* ignore */}
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _saveName() async {
    setState(() => _renaming = false);
    final next = _name.text.trim();
    if (next == (_device.name ?? '')) return;
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref
          .read(bcApiProvider)
          .deviceUpdate({'device_id': _device.deviceId, 'name': next});
      toast.show('Renamed', accent: look.accent);
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  Future<void> _togglePoint(PointRow p, String field, bool current) async {
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref.read(bcApiProvider).deviceUpdate({
        'device_id': _device.deviceId,
        'point_id': p.pointId,
        field: !current,
      });
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  Future<void> _decommission() async {
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref.read(bcApiProvider).decommission([_device.deviceId]);
      toast.show('Decommissioned', accent: look.accent2);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  Future<void> _openLabel() async {
    final toast = ref.read(toastProvider.notifier);
    try {
      final label = await ref.read(bcApiProvider).labelRender(_device.deviceId);
      if (!mounted) return;
      await showGlassBottomSheet<void>(
        context: context,
        title: 'Print label',
        builder: (_) => QrLabel(
          value: label.qrUrl,
          title: label.displayName,
          subtitle: label.serial,
          caption: label.code128,
        ),
      );
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    return Scaffold(
      body: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _Header(
              device: _device,
              renaming: _renaming,
              nameController: _name,
              onBack: () => Navigator.of(context).pop(),
              onStartRename: () => setState(() => _renaming = true),
              onSaveName: _saveName,
            ),
            const SizedBox(height: 20),
            Row(
              children: [
                if (isPlaceable(_device)) ...[
                  Expanded(
                    child: _Action(
                      icon: LucideIcons.mapPin,
                      label: 'Place on page',
                      accent: statusColor('pending', look),
                      onTap: () => showPlaceOnPageSheet(context, ref, _device),
                    ),
                  ),
                  const SizedBox(width: 8),
                ],
                Expanded(
                  child: _Action(
                    icon: LucideIcons.printer,
                    label: 'Label',
                    accent: look.accent,
                    onTap: _openLabel,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _Action(
                    icon: LucideIcons.trash2,
                    label: 'Decommission',
                    accent: look.accent2,
                    onTap: _decommission,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 20),
            SectionLabel('Points · ${_points.length}'),
            for (final p in _points) ...[
              _PointCard(
                point: p,
                accent: look.accent,
                onTrend: () => _togglePoint(p, 'trend_on', p.trendOn),
                onAlarm: () => _togglePoint(p, 'alarm_on', p.alarmOn),
              ),
              const SizedBox(height: 8),
            ],
          ],
        ),
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({
    required this.device,
    required this.renaming,
    required this.nameController,
    required this.onBack,
    required this.onStartRename,
    required this.onSaveName,
  });

  final DeviceRow device;
  final bool renaming;
  final TextEditingController nameController;
  final VoidCallback onBack;
  final VoidCallback onStartRename;
  final VoidCallback onSaveName;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Pressable(
          onTap: onBack,
          scale: 0.92,
          semanticLabel: 'Back to devices',
          child: Container(
            width: 36,
            height: 36,
            decoration: const BoxDecoration(
              color: Color(0x0FFFFFFF),
              shape: BoxShape.circle,
            ),
            child: Icon(LucideIcons.arrowLeft,
                size: 20, color: look.inkSoft),
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (renaming)
                GlassTextField(
                  controller: nameController,
                  semanticLabel: 'Device name',
                  onSubmitted: onSaveName,
                )
              else
                Pressable(
                  onTap: onStartRename,
                  scale: 0.98,
                  semanticLabel: 'Rename device',
                  child: Row(
                    children: [
                      Flexible(
                        child: Text(
                          device.name ?? device.deviceId,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: RubixText.headlineMobile,
                        ),
                      ),
                      const SizedBox(width: 8),
                      Icon(LucideIcons.pencil,
                          size: 16, color: look.inkMuted),
                    ],
                  ),
                ),
              const SizedBox(height: 2),
              Row(
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: statusColor(device.status, look),
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Flexible(
                    child: Text(
                      '${device.template} · ${device.status}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: look.inkSoft,
                        fontSize: 14,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _Action extends StatelessWidget {
  const _Action({
    required this.icon,
    required this.label,
    required this.accent,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return GlassCard(
      onTap: onTap,
      padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 16, color: accent),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: look.ink,
                fontSize: 14,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _PointCard extends StatelessWidget {
  const _PointCard({
    required this.point,
    required this.accent,
    required this.onTrend,
    required this.onAlarm,
  });

  final PointRow point;
  final Color accent;
  final VoidCallback onTrend;
  final VoidCallback onAlarm;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return GlassCard(
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  point.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: look.ink,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  point.unit != null && point.unit!.isNotEmpty
                      ? '${point.widget} · ${point.unit}'
                      : point.widget,
                  style: TextStyle(
                      color: look.inkMuted, fontSize: 12),
                ),
              ],
            ),
          ),
          _ToggleLabeled(
            label: 'Trend',
            on: point.trendOn,
            onToggle: onTrend,
            accent: accent,
          ),
          const SizedBox(width: 16),
          _ToggleLabeled(
            label: 'Alarm',
            on: point.alarmOn,
            onToggle: onAlarm,
            accent: accent,
          ),
        ],
      ),
    );
  }
}

class _ToggleLabeled extends StatelessWidget {
  const _ToggleLabeled({
    required this.label,
    required this.on,
    required this.onToggle,
    required this.accent,
  });

  final String label;
  final bool on;
  final VoidCallback onToggle;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        GlassToggle(
          on: on,
          onToggle: onToggle,
          accent: accent,
          semanticLabel: label,
        ),
        const SizedBox(height: 4),
        Text(label.toUpperCase(), style: RubixText.label),
      ],
    );
  }
}
