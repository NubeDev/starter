import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/demo/demo_mode.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/human_error.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
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
    final listAsync = ref.watch(dashboardListProvider);

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
                'Live across every site. Pinned views below.',
                style: TextStyle(color: t.muted, fontSize: 14, height: 1.45),
              ),
              const SizedBox(height: 22),
              const _StatChipsRow(),
              const SizedBox(height: 14),
              const _EnergyCard(),
              const SizedBox(height: 12),
              const _HealthCard(),
              const SizedBox(height: 22),
              Text(
                'YOUR DASHBOARDS',
                style: TextStyle(
                  color: t.muted,
                  fontSize: 11,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 1.2,
                ),
              ),
              const SizedBox(height: 10),
              listAsync.when(
                loading: () => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: LoadingIndicator(),
                ),
                error: (e, _) {
                  final he = humanizeNetworkError(e);
                  return ErrorPanel(
                    title: 'Could not load dashboards',
                    message: he.body,
                    details: he.details,
                    intent: ErrorPanelIntent.destructive,
                    onRetry: () => ref.invalidate(dashboardListProvider),
                  );
                },
                data: (items) {
                  if (items.isEmpty) {
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 24),
                      child: Center(
                        child: Text(
                          'No dashboards yet',
                          style: TextStyle(color: t.muted, fontSize: 13),
                        ),
                      ),
                    );
                  }
                  return Column(
                    children: [
                      for (final item in items) ...[
                        _DashboardRow(item: item),
                        const SizedBox(height: 8),
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
    return const IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: _StatChip(
              label: 'DEVICES',
              value: '24',
              delta: '+2',
              up: true,
            ),
          ),
          SizedBox(width: 10),
          Expanded(
            child: _StatChip(
              label: 'SITE LOAD',
              value: '67%',
              delta: '-4%',
              up: false,
            ),
          ),
          SizedBox(width: 10),
          Expanded(
            child: _StatChip(
              label: 'LATENCY',
              value: '12ms',
              delta: '-3ms',
              up: false,
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
  });
  final String label;
  final String value;
  final String delta;
  final bool up;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final deltaColor = up ? t.success : t.danger;
    return NubeGlowCard(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: TextStyle(
              color: t.muted,
              fontSize: 10.5,
              fontWeight: FontWeight.w600,
              letterSpacing: 1.1,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            value,
            style: TextStyle(
              color: t.text,
              fontSize: 26,
              fontWeight: FontWeight.w700,
              height: 1,
              letterSpacing: -0.6,
            ),
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Icon(
                up ? LucideIcons.trendingUp : LucideIcons.trendingDown,
                size: 11,
                color: deltaColor,
              ),
              const SizedBox(width: 4),
              Text(
                delta,
                style: TextStyle(
                  color: deltaColor,
                  fontSize: 11.5,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ],
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
    const values = <double>[12, 16, 14, 19, 22, 21, 27, 31, 28, 34, 36, 39];
    const labels = <String>[
      'jan', 'feb', 'mar', 'apr', 'may', 'jun',
      'jul', 'aug', 'sep', 'oct', 'nov', 'dec',
    ];
    return NubeGlowCard(
      tone: NubeGlowTone.teal,
      padding: const EdgeInsets.fromLTRB(18, 18, 18, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  'ENERGY HARVESTED',
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 1.2,
                  ),
                ),
              ),
              Text(
                '432 kWh',
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w700,
                  letterSpacing: -0.2,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          const NubeAreaChart(
            values: values,
            labels: labels,
            tone: NubeGlowTone.teal,
            height: 200,
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
    return NubeGlowCard(
      tone: NubeGlowTone.teal,
      padding: const EdgeInsets.all(18),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          const SizedBox(
            width: 130,
            height: 130,
            child: NubeDonut(
              percent: 94,
              caption: 'HEALTHY',
              tone: NubeGlowTone.teal,
              size: 130,
            ),
          ),
          const SizedBox(width: 18),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'DEVICE HEALTH',
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  '22 of 24 nominal',
                  style: TextStyle(
                    color: t.text,
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    letterSpacing: -0.2,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  '1 warning · 1 offline',
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 12.5,
                    height: 1.4,
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

// ────────────────────────────────────────────────────────────────────────
// Dashboard list row
// ────────────────────────────────────────────────────────────────────────

class _DashboardRow extends StatelessWidget {
  const _DashboardRow({required this.item});
  final DashboardItem item;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return NubeGlowCard(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      onTap: () => context.push('/sdui/${item.pageId}'),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: t.leaf.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(9),
            ),
            alignment: Alignment.center,
            child: Icon(LucideIcons.layoutGrid, size: 16, color: t.leaf),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  item.title,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  item.pageId,
                  style: TextStyle(color: t.muted, fontSize: 11.5),
                ),
              ],
            ),
          ),
          Icon(LucideIcons.chevronRight, size: 16, color: t.muted),
        ],
      ),
    );
  }
}
