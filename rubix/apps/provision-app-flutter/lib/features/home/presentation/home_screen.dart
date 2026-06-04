import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/features/home/data/home_providers.dart';
import 'package:provision_app/features/home/domain/relative_time.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// The app's landing screen — "Home A · Action-led" from Figma. Replaces booting
/// straight into the camera: the scanner only opens when the operator taps
/// "Start scanning". Hero → two action cards → quick-nav counts → recent
/// devices. All navigation is delegated to the shell via callbacks so this
/// screen reuses the real routes.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({
    required this.onScan,
    required this.onAddManually,
    required this.onOpenDevices,
    required this.onOpenSites,
    required this.onOpenTemplates,
    required this.onOpenDevice,
    super.key,
  });

  /// "Start scanning" → the scan flow in camera mode.
  final VoidCallback onScan;

  /// "Add manually" → the scan flow in manual / pick-a-type mode.
  final VoidCallback onAddManually;

  final VoidCallback onOpenDevices;
  final VoidCallback onOpenSites;
  final VoidCallback onOpenTemplates;

  /// Tap a recent row → that device's detail.
  final void Function(DeviceRow device) onOpenDevice;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = context.look;
    final data = ref.watch(homeDataProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Status pill is the global floating TopBar (rendered by AppShell),
          //    so it isn't repeated here — the top padding clears it.

          // 2. Hero — eyebrow + "Provision a device" with "device" in the teal
          //    heading-accent (only the last word is tinted).
          const SectionLabel('Provision'),
          Padding(
            padding: const EdgeInsets.only(bottom: 24),
            child: Text.rich(
              TextSpan(
                children: [
                  const TextSpan(text: 'Provision a '),
                  TextSpan(
                    text: 'device',
                    style: TextStyle(color: look.accent),
                  ),
                ],
              ),
              style: RubixText.headlineMobile,
            ),
          ),

          // 3. Action cards.
          _ActionCard(
            icon: LucideIcons.scanLine,
            title: 'Start scanning',
            subtitle: 'Point at a device QR',
            primary: true,
            onTap: onScan,
          ),
          const SizedBox(height: 12),
          _ActionCard(
            icon: LucideIcons.keyboard,
            title: 'Add manually',
            subtitle: 'Enter device details',
            primary: false,
            onTap: onAddManually,
          ),
          const SizedBox(height: 20),

          // 4. Quick-nav count tiles.
          Row(
            children: [
              Expanded(
                child: _CountTile(
                  icon: LucideIcons.cpu,
                  count: data.value?.deviceCount,
                  label: 'devices',
                  loading: data.isLoading,
                  onTap: onOpenDevices,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: _CountTile(
                  icon: LucideIcons.building2,
                  count: data.value?.siteCount,
                  label: 'sites',
                  loading: data.isLoading,
                  onTap: onOpenSites,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: _CountTile(
                  icon: LucideIcons.layoutGrid,
                  count: data.value?.templateCount,
                  label: 'types',
                  loading: data.isLoading,
                  onTap: onOpenTemplates,
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 5. Recent.
          const SectionLabel('Recent'),
          data.when(
            loading: () => const _RecentSkeleton(),
            error: (e, _) => _RecentMessage('Couldn\'t load recent devices.'),
            data: (d) => d.recent.isEmpty
                ? const _RecentMessage('No devices provisioned yet.')
                : Column(
                    children: [
                      for (final r in d.recent) ...[
                        _RecentRow(recent: r, onTap: () => onOpenDevice(r.device)),
                        if (r != d.recent.last) const SizedBox(height: 8),
                      ],
                    ],
                  ),
          ),
        ],
      ),
    );
  }
}

/// A full-width stacked action card. [primary] gives the teal-tinted treatment
/// (Start scanning); otherwise a neutral glass card (Add manually).
class _ActionCard extends StatelessWidget {
  const _ActionCard({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.primary,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool primary;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final fill = primary ? look.accent.withValues(alpha: 0.10) : Glass.fill;
    final border =
        primary ? look.accent.withValues(alpha: 0.45) : Glass.border;
    final tileFill = primary
        ? look.accent.withValues(alpha: 0.18)
        : Colors.white.withValues(alpha: 0.06);
    final tileBorder =
        primary ? look.accent.withValues(alpha: 0.40) : Colors.transparent;
    final iconColor = primary ? look.accent : look.inkSoft;
    final titleColor = primary ? look.accent : look.ink;

    return Pressable(
      onTap: onTap,
      semanticLabel: title,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 20),
        decoration: BoxDecoration(
          color: fill,
          borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
          border: Border.all(color: border),
        ),
        child: Row(
          children: [
            Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                color: tileFill,
                borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
                border: Border.all(color: tileBorder),
              ),
              child: Icon(icon, size: 22, color: iconColor),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: TextStyle(
                      color: titleColor,
                      fontSize: 18,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 3),
                  Text(
                    subtitle,
                    style: TextStyle(color: look.inkMuted, fontSize: 13),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Icon(LucideIcons.arrowRight, size: 18, color: look.inkMuted),
          ],
        ),
      ),
    );
  }
}

/// A quick-nav tile: teal icon, then a count (or a dash while loading) + label.
/// Routes to its section on tap.
class _CountTile extends StatelessWidget {
  const _CountTile({
    required this.icon,
    required this.count,
    required this.label,
    required this.loading,
    required this.onTap,
  });

  final IconData icon;
  final int? count;
  final String label;
  final bool loading;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return GlassCard(
      onTap: onTap,
      padding: const EdgeInsets.all(14),
      child: SizedBox(
        height: 72,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Icon(icon, size: 18, color: look.accent),
            Row(
              crossAxisAlignment: CrossAxisAlignment.baseline,
              textBaseline: TextBaseline.alphabetic,
              children: [
                Text(
                  loading ? '—' : '${count ?? '—'}',
                  style: TextStyle(
                    color: look.ink,
                    fontSize: 18,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(width: 6),
                Flexible(
                  child: Text(
                    label,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: look.inkMuted, fontSize: 12),
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

/// One recent-device row: green dot + name + place label, relative time at the
/// trailing edge.
class _RecentRow extends StatelessWidget {
  const _RecentRow({required this.recent, required this.onTap});

  final RecentDevice recent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final d = recent.device;
    final time = relativeTimeLabel(recent.provisionedAt);
    return GlassCard(
      onTap: onTap,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      borderRadius: const BorderRadius.all(Radius.circular(RubixTokens.radiusMd)),
      child: Row(
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(
              color: RubixTokens.online,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  d.name ?? d.deviceId,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: look.ink,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                if (recent.placeLabel.isNotEmpty)
                  Text(
                    recent.placeLabel,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(color: look.inkMuted, fontSize: 12),
                  ),
              ],
            ),
          ),
          if (time != null) ...[
            const SizedBox(width: 8),
            Text(time, style: TextStyle(color: look.inkMuted, fontSize: 12)),
          ],
        ],
      ),
    );
  }
}

/// Loading placeholder for the recent list — three dim glass bars.
class _RecentSkeleton extends StatelessWidget {
  const _RecentSkeleton();

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        for (var i = 0; i < 3; i++) ...[
          GlassCard(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
            child: const SizedBox(height: 34),
          ),
          if (i < 2) const SizedBox(height: 8),
        ],
      ],
    );
  }
}

class _RecentMessage extends StatelessWidget {
  const _RecentMessage(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 16),
      child: Text(
        text,
        style: TextStyle(color: context.look.inkMuted, fontSize: 14),
      ),
    );
  }
}
