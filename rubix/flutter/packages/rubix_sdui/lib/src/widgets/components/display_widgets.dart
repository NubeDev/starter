/// Display widgets — `kpi`, `kpi_grid`, `chart`.
///
/// Visual contract matches the React renderer at
/// `packages/starter-ui-sdui-react/src/renderer/`; see
/// `rubix/docs/design/sdui/visual-design-spec.md`.
library;

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../../models/component.dart';
import '../sdui_theme.dart';
import 'accent.dart';

/// Glass card frame shared by KPI and chart widgets. Stacks a
/// radial-gradient glow blob in the top-right corner, a 1px top
/// hairline in the accent color, and the surface fill.
class _SduiGlassCard extends StatelessWidget {
  const _SduiGlassCard({
    required this.accent,
    required this.child,
    this.padding = const EdgeInsets.all(20),
  });

  final SduiAccent accent;
  final Widget child;
  final EdgeInsets padding;

  @override
  Widget build(BuildContext context) {
    final t = SduiTheme.of(context);
    final c = accentColor(context, accent);
    return ClipRRect(
      borderRadius: BorderRadius.circular(24),
      child: Container(
        decoration: BoxDecoration(
          color: t.glassFill,
          borderRadius: BorderRadius.circular(24),
          border: Border.all(color: t.glassBorder, width: 1),
          boxShadow: const [
            BoxShadow(
              color: Color(0x14000000),
              blurRadius: 24,
              offset: Offset(0, 12),
            ),
          ],
        ),
        child: Stack(
          children: [
            // Glow blob — top-right, accent color at 40% opacity.
            Positioned(
              right: -48,
              top: -48,
              child: IgnorePointer(
                child: Container(
                  width: 128,
                  height: 128,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    gradient: RadialGradient(
                      colors: [c.withValues(alpha: 0.4), c.withValues(alpha: 0)],
                    ),
                  ),
                ),
              ),
            ),
            // Top hairline gradient — accent in the middle, fading out.
            Positioned(
              left: padding.left,
              right: padding.right,
              top: 0,
              child: IgnorePointer(
                child: Container(
                  height: 1,
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [
                        c.withValues(alpha: 0),
                        c.withValues(alpha: 0.8),
                        c.withValues(alpha: 0),
                      ],
                    ),
                  ),
                ),
              ),
            ),
            Padding(padding: padding, child: child),
          ],
        ),
      ),
    );
  }
}

class SduiKpiWidget extends StatelessWidget {
  const SduiKpiWidget({super.key, required this.component});
  final KpiComponent component;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final t = SduiTheme.of(context);
    final raw = component.raw;
    final label = raw['label'] as String? ?? '';
    final formatted = _formatValue(raw['value'], raw['format'] as String?);
    final unit = raw['unit_symbol'] as String? ??
        raw['unit'] as String? ?? '';
    final trend = raw['trend'] as String?;
    final accent = resolveAccent(raw);
    final c = accentColor(context, accent);
    final trendColor = _trendColor(trend, t);

    return _SduiGlassCard(
      accent: accent,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: t.subtleText,
              fontWeight: FontWeight.w600,
              letterSpacing: 1.8,
              height: 1.2,
              fontSize: 11,
            ),
          ),
          const SizedBox(height: 12),
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(
                formatted,
                style: theme.textTheme.displaySmall?.copyWith(
                  color: c,
                  fontWeight: FontWeight.w500,
                  letterSpacing: -1.0,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              if (unit.isNotEmpty) ...[
                const SizedBox(width: 6),
                Text(
                  unit,
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: t.mutedText,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ],
          ),
          if (trend != null && trend.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              trend,
              style: theme.textTheme.labelSmall?.copyWith(
                color: trendColor ?? t.mutedText,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ],
      ),
    );
  }

  Color? _trendColor(String? trend, SduiTheme t) {
    if (trend == null || trend.isEmpty) return null;
    if (trend.startsWith('+')) return t.statusOk;
    if (trend.startsWith('-')) return t.statusDanger;
    final lower = trend.toLowerCase();
    if (lower.startsWith('up')) return t.statusOk;
    if (lower.startsWith('down')) return t.statusDanger;
    return null;
  }

  String _formatValue(Object? value, String? format) {
    if (value == null) return '—';
    if (value is num) {
      switch (format) {
        case 'percent':
          return '${value.toStringAsFixed(1)}%';
        case 'number':
          return value.toStringAsFixed(2);
        default:
          return value.toString();
      }
    }
    return value.toString();
  }
}

/// `kpi_grid` — responsive grid of pre-resolved KPI tiles. Mirrors
/// the React renderer at
/// `packages/starter-ui-sdui-react/src/renderer/render-kpi-grid.tsx`.
class SduiKpiGridWidget extends StatelessWidget {
  const SduiKpiGridWidget({super.key, required this.component});
  final KpiGridComponent component;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final t = SduiTheme.of(context);
    final raw = component.raw;
    final items = (raw['items'] as List? ?? const [])
        .whereType<Map>()
        .map((m) => m.cast<String, Object?>())
        .where((m) => m['label'] is String)
        .toList(growable: false);
    final cols = (raw['columns'] is num && (raw['columns'] as num) > 0)
        ? (raw['columns'] as num).toInt()
        : 4;
    if (items.isEmpty) return const SizedBox.shrink();

    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: cols,
        crossAxisSpacing: 16,
        mainAxisSpacing: 16,
        childAspectRatio: 1.7,
      ),
      itemBuilder: (context, i) {
        final item = items[i];
        final explicit = item['accent'] is String || item['intent'] is String;
        final accent = explicit ? resolveAccent(item) : accentByIndex(i);
        final c = accentColor(context, accent);
        final label = item['label'] as String;
        final formatted = _formatValue(item['value'], item['format'] as String?);
        final unit = item['unit_symbol'] as String? ?? '';
        final delta = item['delta'];
        String? deltaLabel;
        Color? deltaColor;
        if (delta is Map) {
          deltaLabel = delta['label'] as String?;
          final direction = delta['direction'];
          if (direction == 'up') deltaColor = t.statusOk;
          if (direction == 'down') deltaColor = t.statusDanger;
        }

        return _SduiGlassCard(
          accent: accent,
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                label,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: t.subtleText,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 1.8,
                  fontSize: 11,
                ),
              ),
              const SizedBox(height: 8),
              Row(
                crossAxisAlignment: CrossAxisAlignment.baseline,
                textBaseline: TextBaseline.alphabetic,
                children: [
                  Flexible(
                    child: FittedBox(
                      fit: BoxFit.scaleDown,
                      alignment: Alignment.centerLeft,
                      child: Text(
                        formatted,
                        style: theme.textTheme.headlineMedium?.copyWith(
                          color: c,
                          fontWeight: FontWeight.w500,
                          letterSpacing: -0.8,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ),
                  ),
                  if (unit.isNotEmpty) ...[
                    const SizedBox(width: 4),
                    Text(
                      unit,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: t.mutedText,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ],
                ],
              ),
              if (deltaLabel != null && deltaLabel.isNotEmpty) ...[
                const SizedBox(height: 6),
                Text(
                  deltaLabel,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: deltaColor ?? t.mutedText,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  String _formatValue(Object? value, String? format) {
    if (value == null) return '—';
    if (value is num) {
      switch (format) {
        case 'percent':
          return '${value.toStringAsFixed(1)}%';
        case 'number':
          return value.toStringAsFixed(2);
        default:
          return value.toString();
      }
    }
    return value.toString();
  }
}

class SduiChartWidget extends StatelessWidget {
  const SduiChartWidget({super.key, required this.component});
  final ChartComponent component;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final t = SduiTheme.of(context);
    final raw = component.raw;
    final title = raw['title'] as String? ?? '';
    final isSparkline = raw['type'] == 'sparkline';
    final seriesList = _extractSeries(raw);
    final gridLine = t.glassBorder;
    final axisColor = t.mutedText;

    // Frame accent cycles through the palette by chart id hash so two
    // charts on the same page get different accent halos. Series
    // strokes use the same five-color rotation per series index.
    final frameAccent = resolveAccent(raw);

    final bars = <LineChartBarData>[];
    for (var i = 0; i < seriesList.length; i++) {
      final color = accentColor(context, accentByIndex(i));
      final spots = <FlSpot>[];
      for (final p in seriesList[i]) {
        spots.add(FlSpot(p.$1.toDouble(), p.$2.toDouble()));
      }
      bars.add(LineChartBarData(
        spots: spots,
        isCurved: false,
        color: color,
        barWidth: 2,
        dotData: const FlDotData(show: false),
        belowBarData: BarAreaData(
          show: true,
          color: color.withValues(alpha: 0.14),
        ),
      ));
    }

    final height = isSparkline ? 64.0 : 220.0;

    return _SduiGlassCard(
      accent: frameAccent,
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          if (title.isNotEmpty) ...[
            Text(
              title,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
                letterSpacing: -0.2,
              ),
            ),
            const SizedBox(height: 12),
          ],
          SizedBox(
            height: height,
            child: bars.isEmpty
                ? Container(
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(16),
                      border: Border.all(
                        color: axisColor.withValues(alpha: 0.4),
                        style: BorderStyle.solid,
                      ),
                    ),
                    alignment: Alignment.center,
                    child: Text(
                      'no data',
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: t.mutedText),
                    ),
                  )
                : LineChart(
                    LineChartData(
                      lineBarsData: bars,
                      titlesData: FlTitlesData(
                        leftTitles: AxisTitles(
                          sideTitles: SideTitles(
                            showTitles: !isSparkline,
                            reservedSize: 44,
                            getTitlesWidget: (v, meta) => Padding(
                              padding: const EdgeInsets.only(right: 6),
                              child: Text(
                                meta.formattedValue,
                                style: TextStyle(
                                  color: axisColor,
                                  fontSize: 11,
                                ),
                              ),
                            ),
                          ),
                        ),
                        bottomTitles: AxisTitles(
                          sideTitles: SideTitles(
                            showTitles: !isSparkline,
                            reservedSize: 22,
                            getTitlesWidget: (v, meta) => Text(
                              meta.formattedValue,
                              style: TextStyle(
                                color: axisColor,
                                fontSize: 11,
                              ),
                            ),
                          ),
                        ),
                        rightTitles: const AxisTitles(
                          sideTitles: SideTitles(showTitles: false),
                        ),
                        topTitles: const AxisTitles(
                          sideTitles: SideTitles(showTitles: false),
                        ),
                      ),
                      gridData: FlGridData(
                        show: !isSparkline,
                        drawVerticalLine: false,
                        getDrawingHorizontalLine: (v) => FlLine(
                          color: gridLine,
                          strokeWidth: 1,
                        ),
                      ),
                      borderData: FlBorderData(show: false),
                    ),
                  ),
          ),
        ],
      ),
    );
  }

  /// Walks `raw['series']` (or `raw['sources']`) into a list of point
  /// lists. Each entry is a `(timestamp, value)` record.
  List<List<(num, num)>> _extractSeries(Map<String, Object?> raw) {
    final List candidates = (raw['series'] is List && (raw['series'] as List).isNotEmpty)
        ? raw['series'] as List
        : (raw['sources'] is List ? raw['sources'] as List : const []);
    final out = <List<(num, num)>>[];
    for (final s in candidates) {
      if (s is! Map) continue;
      final points = s['points'];
      if (points is! List) continue;
      final ps = <(num, num)>[];
      for (final p in points) {
        if (p is List && p.length >= 2 && p[0] is num && p[1] is num) {
          ps.add(((p[0] as num), (p[1] as num)));
        }
      }
      if (ps.isNotEmpty) out.add(ps);
    }
    return out;
  }
}
