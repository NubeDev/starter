/// Display widgets — `kpi`, `chart`.
///
/// Wave 1 subset for the proof slice (docs/PROOF.md). `text`,
/// `heading`, `badge`, `markdown`, `kpi_grid`, `sparkline`, `diff`,
/// `code`, `icon`, `image` land next.
library;

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../../models/component.dart';

class SduiKpiWidget extends StatelessWidget {
  const SduiKpiWidget({super.key, required this.component});
  final KpiComponent component;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final raw = component.raw;
    final label = raw['label'] as String? ?? '';
    final formatted = _formatValue(raw['value'], raw['format'] as String?);
    final unit = raw['unit_symbol'] as String? ?? '';

    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: theme.colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              label,
              style: theme.textTheme.labelLarge?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 8),
            Row(
              crossAxisAlignment: CrossAxisAlignment.baseline,
              textBaseline: TextBaseline.alphabetic,
              children: [
                Text(formatted, style: theme.textTheme.headlineMedium),
                if (unit.isNotEmpty) ...[
                  const SizedBox(width: 6),
                  Text(unit, style: theme.textTheme.titleMedium),
                ],
              ],
            ),
          ],
        ),
      ),
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
    final raw = component.raw;
    final title = raw['title'] as String? ?? '';
    final series = _firstSeries(raw);

    final spots = <FlSpot>[];
    if (series != null) {
      final points = series['points'];
      if (points is List) {
        for (final p in points) {
          if (p is List && p.length >= 2 && p[0] is num && p[1] is num) {
            spots.add(FlSpot(
              (p[0] as num).toDouble(),
              (p[1] as num).toDouble(),
            ));
          }
        }
      }
    }

    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: theme.colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (title.isNotEmpty) ...[
              Text(title, style: theme.textTheme.titleMedium),
              const SizedBox(height: 12),
            ],
            SizedBox(
              height: 240,
              child: spots.isEmpty
                  ? Center(
                      child: Text(
                        'No data',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    )
                  : LineChart(
                      LineChartData(
                        lineBarsData: [
                          LineChartBarData(
                            spots: spots,
                            isCurved: false,
                            color: theme.colorScheme.primary,
                            barWidth: 2,
                            dotData: const FlDotData(show: false),
                          ),
                        ],
                        titlesData: const FlTitlesData(
                          leftTitles: AxisTitles(
                            sideTitles: SideTitles(
                                showTitles: true, reservedSize: 44),
                          ),
                          bottomTitles: AxisTitles(
                            sideTitles: SideTitles(showTitles: false),
                          ),
                          rightTitles: AxisTitles(
                            sideTitles: SideTitles(showTitles: false),
                          ),
                          topTitles: AxisTitles(
                            sideTitles: SideTitles(showTitles: false),
                          ),
                        ),
                        gridData: const FlGridData(show: true),
                        borderData: FlBorderData(show: false),
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }

  Map<String, Object?>? _firstSeries(Map<String, Object?> raw) {
    final s = raw['series'];
    if (s is! List || s.isEmpty) return null;
    final first = s.first;
    if (first is Map) return first.cast<String, Object?>();
    return null;
  }
}
