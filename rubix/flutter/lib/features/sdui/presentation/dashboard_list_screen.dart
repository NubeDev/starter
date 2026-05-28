import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/demo/demo_mode.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';
import 'package:rubix_flutter/shared/widgets/scaffold/ambient_glow_background.dart';

/// Lists dashboards for the `system` tenant via
/// `POST /api/v1/tools/rubix.dashboard.list` — or returns the canned
/// demo list when `--dart-define=DEMO_MODE=true` is active.
final dashboardListProvider =
    FutureProvider.autoDispose<List<DashboardItem>>((ref) async {
  if (demoMode) {
    return [
      for (final d in kDemoDashboards)
        DashboardItem(pageId: d.pageId, title: d.title),
    ];
  }
  final auth = ref.watch(authControllerProvider).value;
  if (auth is! AuthAuthenticated) {
    throw StateError('Not signed in to this connection.');
  }
  final dio = ref.watch(dioProvider);
  if (dio == null) {
    throw StateError('No active connection — add one in Connections first.');
  }
  final res = await dio.post<Map<String, dynamic>>(
    '/api/v1/tools/rubix.dashboard.list',
    data: const {'tenant_id': 'system'},
  );
  final raw = (res.data ?? const <String, dynamic>{})['items'];
  final parsed = <DashboardItem>[];
  if (raw is List) {
    for (final item in raw) {
      if (item is Map) {
        parsed.add(DashboardItem.fromJson(item.cast<String, Object?>()));
      }
    }
  }
  parsed.sort((a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()));
  return parsed;
});

class DashboardItem {
  const DashboardItem({required this.pageId, required this.title});
  factory DashboardItem.fromJson(Map<String, Object?> map) => DashboardItem(
        pageId: map['page_id'] as String? ?? '',
        title: map['title'] as String? ?? '',
      );
  final String pageId;
  final String title;
}

/// Dashboards — Figma-aligned: hero with serif-italic accent, three
/// stat chips, energy area chart, device-health donut, then the list.
class DashboardListScreen extends ConsumerWidget {
  const DashboardListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final t = Theme.of(context).nube;

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
            children: [
              const SizedBox(height: 6),
              const _LiveGlassPill(),
              const SizedBox(height: 18),
              const _Hero(),
              const SizedBox(height: 10),
              Text(
                'Updated just now · 3 sites',
                style: TextStyle(color: t.muted, fontSize: 13, height: 1.45),
              ),
              const SizedBox(height: 22),
              const _StatChipsRow(),
              const SizedBox(height: 12),
              const _EnergyCard(),
              const SizedBox(height: 12),
              const _HealthCard(),
            ],
          ),
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
            text: 'Fleet at a\n',
            style: TextStyle(
              color: t.text,
              fontSize: 38,
              fontWeight: FontWeight.w600,
              height: 1.05,
              letterSpacing: -0.8,
            ),
          ),
          TextSpan(text: 'glance.', style: italic),
        ],
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Stat chips row
// ────────────────────────────────────────────────────────────────────────

class _StatChipsRow extends StatelessWidget {
  const _StatChipsRow();
  @override
  Widget build(BuildContext context) {
    // Delta colour is SEMANTIC — `good` says whether the change is
    // positive for the user. Arrow direction is independent. For
    // latency, a *down* arrow is good, so `good: true` + `up: false`.
    return const IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: _StatChip(
              label: 'DEVICES',
              value: '412',
              delta: '2.4%',
              up: true,
              good: true,
            ),
          ),
          SizedBox(width: 12),
          Expanded(
            child: _StatChip(
              label: 'SITE LOAD',
              value: '2.1k',
              delta: '1.2%',
              up: false,
              good: false,
            ),
          ),
          SizedBox(width: 12),
          Expanded(
            child: _StatChip(
              label: 'LATENCY',
              value: '42ms',
              delta: '8.5%',
              up: false,
              good: true,
            ),
          ),
        ],
      ),
    );
  }
}

class _StatChip extends StatelessWidget {
  const _StatChip({
    required this.label,
    required this.value,
    required this.delta,
    required this.up,
    required this.good,
  });
  final String label;
  final String value;
  final String delta;
  /// Arrow direction (visual only).
  final bool up;
  /// Whether the change is good for the user (drives colour).
  final bool good;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final deltaColor = good ? t.leaf : t.danger;
    return SizedBox(
      height: 84,
      child: NubeGlowCard(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              label,
              style: TextStyle(
                color: t.muted,
                fontSize: 10,
                fontWeight: FontWeight.w600,
                letterSpacing: 1.2,
              ),
            ),
            Text(
              value,
              style: TextStyle(
                color: t.text,
                fontSize: 24,
                fontWeight: FontWeight.w600,
                height: 1,
                letterSpacing: -0.6,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  up ? LucideIcons.arrowUpRight : LucideIcons.arrowDownRight,
                  size: 11,
                  color: deltaColor,
                ),
                const SizedBox(width: 3),
                Text(
                  delta,
                  style: TextStyle(
                    color: deltaColor,
                    fontSize: 11,
                    fontWeight: FontWeight.w400,
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

// ────────────────────────────────────────────────────────────────────────
// Energy area chart
// ────────────────────────────────────────────────────────────────────────

class _EnergyCard extends StatelessWidget {
  const _EnergyCard();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    // 7-day series, Mon–Sun — the X-axis labels below match.
    const values = <double>[28, 34, 31, 38, 42, 39, 42.3];
    const labels = <String>['M', 'T', 'W', 'T', 'F', 'S', 'S'];
    return NubeGlowCard(
      tone: NubeGlowTone.teal,
      padding: const EdgeInsets.fromLTRB(18, 16, 18, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'ENERGY HARVESTED',
                      style: TextStyle(
                        color: t.muted,
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                        letterSpacing: 1.2,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.end,
                      children: [
                        Text(
                          '42.3',
                          style: TextStyle(
                            color: t.text,
                            fontSize: 30,
                            fontWeight: FontWeight.w600,
                            height: 1,
                            letterSpacing: -0.8,
                            fontFeatures: const [FontFeature.tabularFigures()],
                          ),
                        ),
                        const SizedBox(width: 6),
                        Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Text(
                            'kWh',
                            style: TextStyle(
                              color: t.muted,
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    // Yellow accent underline — node 6:61. Underline only.
                    Container(
                      width: 40,
                      height: 2,
                      decoration: BoxDecoration(
                        color: t.callout,
                        borderRadius: BorderRadius.circular(2),
                      ),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      LucideIcons.arrowUpRight,
                      size: 12,
                      color: t.leaf,
                    ),
                    const SizedBox(width: 3),
                    Text(
                      '12.4%',
                      style: TextStyle(
                        color: t.leaf,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          const NubeAreaChart(
            values: values,
            labels: labels,
            tone: NubeGlowTone.teal,
            height: 132,
          ),
        ],
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Device-health donut
// ────────────────────────────────────────────────────────────────────────

class _HealthCard extends StatelessWidget {
  const _HealthCard();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return SizedBox(
      height: 96,
      child: NubeGlowCard(
        tone: NubeGlowTone.teal,
        padding: const EdgeInsets.fromLTRB(18, 14, 18, 14),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'DEVICE HEALTH',
                    style: TextStyle(
                      color: t.muted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 1.2,
                    ),
                  ),
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.baseline,
                    textBaseline: TextBaseline.alphabetic,
                    children: [
                      Text(
                        '94%',
                        style: TextStyle(
                          color: t.text,
                          fontSize: 28,
                          fontWeight: FontWeight.w600,
                          height: 1,
                          letterSpacing: -0.6,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                      const SizedBox(width: 6),
                      Padding(
                        padding: const EdgeInsets.only(bottom: 2),
                        child: Text(
                          'online',
                          style: TextStyle(
                            color: t.muted,
                            fontSize: 13,
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            const SizedBox(
              width: 64,
              height: 64,
              child: NubeDonut(
                percent: 94,
                caption: '',
                tone: NubeGlowTone.teal,
                size: 64,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────
// Live glass pill — green dot + LIVE label, node 6:38–6:40.
// ─────────────────────────────────────────────────────────────────────

class _LiveGlassPill extends StatelessWidget {
  const _LiveGlassPill();

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
              'LIVE',
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


