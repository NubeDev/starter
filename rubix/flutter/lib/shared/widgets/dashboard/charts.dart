import 'dart:math' as math;

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

// ---------------------------------------------------------------------------
// Helper: resolve tone → line color from the theme.
// ---------------------------------------------------------------------------
Color _toneColor(BuildContext context, NubeGlowTone tone) {
  final t = Theme.of(context).nube;
  switch (tone) {
    case NubeGlowTone.green:
      return t.success;
    case NubeGlowTone.amber:
      return t.warning;
    case NubeGlowTone.danger:
      return t.danger;
    case NubeGlowTone.none:
    case NubeGlowTone.teal:
      return t.leaf;
  }
}

// ---------------------------------------------------------------------------
// NubeMiniSparkline — axis-less area chart used inside KPI tiles.
// ---------------------------------------------------------------------------
class NubeMiniSparkline extends StatelessWidget {
  const NubeMiniSparkline({
    required this.values,
    this.tone = NubeGlowTone.teal,
    this.height = 56,
    super.key,
  });

  final List<double> values;
  final NubeGlowTone tone;
  final double height;

  @override
  Widget build(BuildContext context) {
    if (values.length < 2) return SizedBox(height: height);
    final color = _toneColor(context, tone);
    final spots = [
      for (var i = 0; i < values.length; i++) FlSpot(i.toDouble(), values[i]),
    ];
    return SizedBox(
      height: height,
      child: LineChart(
        LineChartData(
          gridData: const FlGridData(show: false),
          titlesData: const FlTitlesData(show: false),
          borderData: FlBorderData(show: false),
          lineTouchData: const LineTouchData(enabled: false),
          minY: values.reduce(math.min) -
              (values.reduce(math.max) - values.reduce(math.min)) * 0.05,
          lineBarsData: [
            LineChartBarData(
              spots: spots,
              isCurved: true,
              curveSmoothness: 0.32,
              color: color,
              barWidth: 1.8,
              isStrokeCapRound: true,
              dotData: const FlDotData(show: false),
              belowBarData: BarAreaData(
                show: true,
                gradient: LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [
                    color.withValues(alpha: 0.32),
                    color.withValues(alpha: 0.02),
                  ],
                ),
              ),
            ),
          ],
        ),
        duration: const Duration(milliseconds: 700),
        curve: Curves.easeOutCubic,
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeAreaChart — large area chart with axis labels and grid hairlines.
// ---------------------------------------------------------------------------
class NubeAreaChart extends StatelessWidget {
  const NubeAreaChart({
    required this.values,
    required this.labels,
    this.tone = NubeGlowTone.green,
    this.height = 260,
    super.key,
  });

  final List<double> values;
  final List<String> labels;
  final NubeGlowTone tone;
  final double height;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final color = _toneColor(context, tone);
    final spots = [
      for (var i = 0; i < values.length; i++) FlSpot(i.toDouble(), values[i]),
    ];
    final maxV = values.reduce(math.max);
    final minV = values.reduce(math.min);
    final pad = (maxV - minV) * 0.15;

    return SizedBox(
      height: height,
      child: LineChart(
        LineChartData(
          minY: minV - pad,
          maxY: maxV + pad,
          gridData: const FlGridData(show: false),
          titlesData: FlTitlesData(
            leftTitles: const AxisTitles(
              sideTitles: SideTitles(showTitles: false),
            ),
            rightTitles: const AxisTitles(
              sideTitles: SideTitles(showTitles: false),
            ),
            topTitles: const AxisTitles(
              sideTitles: SideTitles(showTitles: false),
            ),
            bottomTitles: AxisTitles(
              sideTitles: SideTitles(
                showTitles: true,
                reservedSize: 26,
                interval: 1,
                getTitlesWidget: (value, _) {
                  final i = value.toInt();
                  if (i < 0 || i >= labels.length) {
                    return const SizedBox.shrink();
                  }
                  return Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: Text(
                      labels[i].toUpperCase(),
                      style: TextStyle(
                        color: t.muted,
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                        letterSpacing: 1.2,
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
          borderData: FlBorderData(show: false),
          lineTouchData: LineTouchData(
            touchTooltipData: LineTouchTooltipData(
              getTooltipColor: (_) => t.surface2,
              getTooltipItems: (touched) => [
                for (final s in touched)
                  LineTooltipItem(
                    s.y.toStringAsFixed(1),
                    TextStyle(
                      color: t.text,
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
              ],
            ),
          ),
          lineBarsData: [
            LineChartBarData(
              spots: spots,
              isCurved: true,
              curveSmoothness: 0.32,
              color: color,
              barWidth: 2,
              isStrokeCapRound: true,
              dotData: const FlDotData(show: false),
              belowBarData: BarAreaData(
                show: true,
                gradient: LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [
                    color.withValues(alpha: 0.28),
                    color.withValues(alpha: 0.02),
                  ],
                ),
              ),
            ),
          ],
        ),
        duration: const Duration(milliseconds: 900),
        curve: Curves.easeOutCubic,
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeDonut — concentric donut with center percentage + caption.
// ---------------------------------------------------------------------------
class NubeDonut extends StatelessWidget {
  const NubeDonut({
    required this.percent,
    this.caption = 'ONLINE',
    this.tone = NubeGlowTone.teal,
    this.size = 180,
    super.key,
  });

  /// 0..100.
  final double percent;
  final String caption;
  final NubeGlowTone tone;
  final double size;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final color = _toneColor(context, tone);
    final pct = percent.clamp(0.0, 100.0);
    return SizedBox(
      width: size,
      height: size,
      child: Stack(
        alignment: Alignment.center,
        children: [
          PieChart(
            PieChartData(
              startDegreeOffset: -90,
              sectionsSpace: 0,
              centerSpaceRadius: size * 0.36,
              sections: [
                PieChartSectionData(
                  value: pct,
                  color: color,
                  radius: size * 0.10,
                  showTitle: false,
                ),
                PieChartSectionData(
                  value: 100 - pct,
                  color: t.border,
                  radius: size * 0.10,
                  showTitle: false,
                ),
              ],
            ),
            duration: const Duration(milliseconds: 800),
            curve: Curves.easeOutCubic,
          ),
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              RichText(
                text: TextSpan(
                  style: TextStyle(
                    color: t.text,
                    fontSize: size * 0.22,
                    fontWeight: FontWeight.w700,
                    height: 1,
                    letterSpacing: -0.5,
                  ),
                  children: [
                    TextSpan(text: pct.toStringAsFixed(0)),
                    TextSpan(
                      text: '%',
                      style: TextStyle(
                        color: t.muted,
                        fontSize: size * 0.12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 6),
              Text(
                caption.toUpperCase(),
                style: TextStyle(
                  color: t.muted,
                  fontSize: 10.5,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 1.4,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
