import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Sparkline trend — a polyline over a deterministic demo series. `seed` keeps
/// each tile's shape stable across renders (no Random). Ported from the React
/// `LineWidget`.
class LineWidget extends StatelessWidget {
  const LineWidget({
    required this.title,
    required this.accent,
    this.unit,
    this.seed = 0,
    super.key,
  });

  final String title;
  final String? unit;
  final Color accent;
  final int seed;

  /// Deterministic pseudo-series from a seed — stable demo shape. Ported
  /// exactly from the React `series(seed)`.
  static List<int> series(int seed) {
    final out = <int>[];
    var v = 40 + (seed % 20).toDouble();
    for (var i = 0; i < 16; i++) {
      v += math.sin((i + seed) * 0.9) * 6 + math.cos((i + seed) * 0.4) * 3;
      out.add(v.clamp(5, 95).round());
    }
    return out;
  }

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final points = series(seed);
    final last = points.last;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(title.toUpperCase(), style: RubixText.label),
            Text.rich(
              TextSpan(
                text: '$last',
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: look.ink,
                ),
                children: [
                  if (unit != null)
                    TextSpan(
                      text: ' $unit',
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w500,
                        color: look.inkMuted,
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        SizedBox(
          height: 48,
          child: CustomPaint(
            size: Size.infinite,
            painter: _SparklinePainter(points: points, accent: accent),
          ),
        ),
      ],
    );
  }
}

class _SparklinePainter extends CustomPainter {
  _SparklinePainter({required this.points, required this.accent});

  final List<int> points;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    if (points.length < 2) return;
    final maxV = points.reduce(math.max);
    final minV = points.reduce(math.min);
    final span = (maxV - minV) == 0 ? 1 : (maxV - minV);

    final path = Path();
    for (var i = 0; i < points.length; i++) {
      final x = (i / (points.length - 1)) * size.width;
      final y = size.height - ((points[i] - minV) / span) * size.height;
      if (i == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }

    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.5
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round
      ..color = accent;
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(_SparklinePainter old) =>
      old.accent != accent || old.points != points;
}
