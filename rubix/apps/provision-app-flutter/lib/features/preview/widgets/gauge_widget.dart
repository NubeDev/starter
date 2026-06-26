import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Radial half-circle gauge — fills an accent arc to `value` over `[min,max]`.
/// Ported from the React `GaugeWidget` (SVG arc). Demo value is passed in.
class GaugeWidget extends StatelessWidget {
  const GaugeWidget({
    required this.title,
    required this.accent,
    this.unit,
    this.value = 0,
    this.min = 0,
    this.max = 100,
    super.key,
  });

  final String title;
  final String? unit;
  final double value;
  final double min;
  final double max;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final range = max - min;
    final pct = ((value - min) / (range == 0 ? 1 : range)).clamp(0.0, 1.0);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 160),
          child: AspectRatio(
            aspectRatio: 100 / 60,
            child: CustomPaint(
              painter: _GaugePainter(pct: pct, accent: accent),
            ),
          ),
        ),
        Transform.translate(
          offset: const Offset(0, -8),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text.rich(
                TextSpan(
                  text: '${value % 1 == 0 ? value.toInt() : value}',
                  style: TextStyle(
                    fontSize: 24,
                    height: 1,
                    fontWeight: FontWeight.w800,
                    color: look.ink,
                  ),
                  children: [
                    if (unit != null)
                      TextSpan(
                        text: ' $unit',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: look.inkMuted,
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(height: 2),
              Text(title.toUpperCase(), style: RubixText.label),
            ],
          ),
        ),
      ],
    );
  }
}

class _GaugePainter extends CustomPainter {
  _GaugePainter({required this.pct, required this.accent});

  final double pct;
  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    // Mirror the React viewBox 0 0 100 60 arc, scaled to the paint size.
    final sx = size.width / 100;
    final sy = size.height / 60;
    final scale = math.min(sx, sy);
    final center = Offset(50 * sx, 56 * sy);
    final radius = 42 * scale;
    final rect = Rect.fromCircle(center: center, radius: radius);
    // Half circle from left (180°/pi) sweeping clockwise to the right (0°).
    const start = math.pi;
    const sweep = math.pi;

    final bg = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 8 * scale
      ..strokeCap = StrokeCap.round
      ..color = Colors.white.withValues(alpha: 0.1);
    canvas.drawArc(rect, start, sweep, false, bg);

    if (pct > 0) {
      final fg = Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 8 * scale
        ..strokeCap = StrokeCap.round
        ..color = accent;
      canvas.drawArc(rect, start, sweep * pct, false, fg);
    }
  }

  @override
  bool shouldRepaint(_GaugePainter old) =>
      old.pct != pct || old.accent != accent;
}
