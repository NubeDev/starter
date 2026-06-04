import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Status LED — a glowing dot for a boolean/discrete point (on/off, ok/fault).
/// Ported from the React `LedWidget`.
class LedWidget extends StatelessWidget {
  const LedWidget({
    required this.title,
    required this.accent,
    this.on = false,
    super.key,
  });

  final String title;
  final bool on;
  final Color accent;

  static const _muted = Color(0xFF7C8A8A);

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    final color = on ? accent : _muted;
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: color,
            boxShadow: on
                ? [
                    BoxShadow(
                      color: accent,
                      blurRadius: 18,
                      spreadRadius: 2,
                    ),
                  ]
                : null,
          ),
        ),
        const SizedBox(height: 12),
        Text(title.toUpperCase(), style: RubixText.label),
        const SizedBox(height: 12),
        Text(
          on ? 'On' : 'Off',
          style: TextStyle(
            fontSize: 14,
            fontWeight: FontWeight.w600,
            color: look.inkSoft,
          ),
        ),
      ],
    );
  }
}
