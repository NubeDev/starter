import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/theme/app_themes.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';

/// Animated ambient backdrop = the theme's base gradient + an optional status
/// tint layered on top. Cross-fades whenever the theme or live status changes,
/// and the washes slowly DRIFT — a long, looping breathing motion that keeps the
/// DNA glow feeling alive instead of static. Ported from the React
/// `MoodBackdrop`; sits behind all page content, filling the frame with
/// [Look.base].
class MoodBackdrop extends ConsumerStatefulWidget {
  const MoodBackdrop({super.key});

  @override
  ConsumerState<MoodBackdrop> createState() => _MoodBackdropState();
}

class _MoodBackdropState extends ConsumerState<MoodBackdrop>
    with SingleTickerProviderStateMixin {
  late final AnimationController _drift;

  @override
  void initState() {
    super.initState();
    // One slow loop (~18s) drives a gentle elliptical drift of every wash. Long
    // enough to read as ambient, not animated.
    _drift = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 18),
    )..repeat();
  }

  @override
  void dispose() {
    _drift.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final washes = [
      ...look.baseGradient,
      if (look.statusTint != null) look.statusTint!,
    ];

    return Positioned.fill(
      child: ColoredBox(
        color: look.base,
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 700),
          child: RepaintBoundary(
            key: ValueKey(look.backdropKey),
            child: AnimatedBuilder(
              animation: _drift,
              builder: (context, _) => CustomPaint(
                painter: _WashPainter(washes, _drift.value),
                size: Size.infinite,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _WashPainter extends CustomPainter {
  _WashPainter(this.washes, this.t);
  final List<RadialWash> washes;

  /// Drift phase in [0,1).
  final double t;

  @override
  void paint(Canvas canvas, Size size) {
    final shorter = size.shortestSide;
    final phase = t * 2 * math.pi;
    for (var i = 0; i < washes.length; i++) {
      final w = washes[i];
      // Each wash drifts on a small ellipse, phase-offset so they don't move in
      // lockstep. Amplitude is a few % of the short side — subtle.
      final off = phase + i * (2 * math.pi / 3);
      final dx = math.cos(off) * shorter * 0.05;
      final dy = math.sin(off * 0.8) * shorter * 0.04;
      final center = Offset(
        (w.alignment.x + 1) / 2 * size.width + dx,
        (w.alignment.y + 1) / 2 * size.height + dy,
      );
      final radius = w.radius * shorter;
      final paint = Paint()
        ..shader = RadialGradient(
          colors: [w.color, w.color.withValues(alpha: 0)],
          stops: [0.0, w.stop],
        ).createShader(Rect.fromCircle(center: center, radius: radius));
      canvas.drawRect(Offset.zero & size, paint);
    }
  }

  @override
  bool shouldRepaint(_WashPainter oldDelegate) =>
      oldDelegate.t != t || !identical(oldDelegate.washes, washes);
}
