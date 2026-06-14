import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';

/// Monotonic counter (pulses, kWh ticks, etc.) — monospace numeric readout with
/// thousands separators. Ported from the React `CounterWidget`.
class CounterWidget extends StatelessWidget {
  const CounterWidget({
    required this.title,
    required this.accent,
    this.unit,
    this.value = 0,
    super.key,
  });

  final String title;
  final String? unit;
  final num value;
  final Color accent;

  /// Simple thousands grouping (e.g. 12345 → "12,345"), integer part only.
  static String _grouped(num v) {
    final negative = v < 0;
    final digits = v.abs().toInt().toString();
    final buf = StringBuffer();
    for (var i = 0; i < digits.length; i++) {
      if (i > 0 && (digits.length - i) % 3 == 0) buf.write(',');
      buf.write(digits[i]);
    }
    return negative ? '-$buf' : buf.toString();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(title.toUpperCase(), style: RubixText.label),
            Icon(LucideIcons.hash, size: 20, color: accent),
          ],
        ),
        Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            Text(
              _grouped(value),
              style: const TextStyle(
                fontFamily: 'monospace',
                fontSize: 28,
                height: 1,
                fontWeight: FontWeight.w700,
                fontFeatures: [FontFeature.tabularFigures()],
                color: RubixTokens.ink,
              ),
            ),
            if (unit != null)
              Padding(
                padding: const EdgeInsets.only(left: 4),
                child: Text(
                  unit!,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: RubixTokens.inkMuted,
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}
