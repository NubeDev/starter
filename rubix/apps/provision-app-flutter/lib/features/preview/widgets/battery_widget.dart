import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Battery level — horizontal fill bar + icon. Turns amber when low (≤20%).
/// Ported from the React `BatteryWidget`.
class BatteryWidget extends StatelessWidget {
  const BatteryWidget({
    required this.title,
    required this.accent,
    this.value = 0,
    super.key,
  });

  final String title;
  final double value;
  final Color accent;

  static const _amber = Color(0xFFFFC24B);

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final pct = value.clamp(0.0, 100.0);
    final low = pct <= 20;
    final color = low ? _amber : accent;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(title.toUpperCase(), style: RubixText.label),
            Icon(
              low ? LucideIcons.batteryLow : LucideIcons.batteryFull,
              size: 20,
              color: color,
            ),
          ],
        ),
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text(
                '${pct.round()}%',
                style: TextStyle(
                  fontSize: 24,
                  fontWeight: FontWeight.w800,
                  color: look.ink,
                ),
              ),
            ),
            ClipRRect(
              borderRadius: BorderRadius.circular(999),
              child: Container(
                height: 8,
                color: Colors.white.withValues(alpha: 0.1),
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: TweenAnimationBuilder<double>(
                    tween: Tween(begin: 0, end: pct / 100),
                    duration: const Duration(milliseconds: 700),
                    curve: Curves.easeOutCubic,
                    builder: (context, t, _) => FractionallySizedBox(
                      widthFactor: t,
                      child: Container(
                        decoration: BoxDecoration(
                          color: color,
                          borderRadius: BorderRadius.circular(999),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
