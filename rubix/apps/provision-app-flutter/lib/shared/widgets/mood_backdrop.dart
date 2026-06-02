import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/theme/app_themes.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';

/// Animated ambient backdrop = the theme's base gradient + an optional status
/// tint layered on top. Cross-fades whenever the theme or live status changes.
/// Ported from the React `MoodBackdrop` (mood → device status). Sits behind all
/// page content, filling the frame with [Look.base].
class MoodBackdrop extends ConsumerWidget {
  const MoodBackdrop({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
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
          child: CustomPaint(
            key: ValueKey(look.backdropKey),
            painter: _WashPainter(washes),
            size: Size.infinite,
          ),
        ),
      ),
    );
  }
}

class _WashPainter extends CustomPainter {
  _WashPainter(this.washes);
  final List<RadialWash> washes;

  @override
  void paint(Canvas canvas, Size size) {
    final shorter = size.shortestSide;
    for (final w in washes) {
      final center = Offset(
        (w.alignment.x + 1) / 2 * size.width,
        (w.alignment.y + 1) / 2 * size.height,
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
      !identical(oldDelegate.washes, washes);
}
