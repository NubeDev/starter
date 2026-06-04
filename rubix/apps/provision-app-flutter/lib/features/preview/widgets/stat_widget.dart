import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Big-number stat tile with a corner glow puddle — the GlanceTile pattern
/// repurposed for a device reading. Ported from the React `StatWidget`.
class StatWidget extends StatelessWidget {
  const StatWidget({
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

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return ClipRect(
      child: Stack(
        children: [
          // Corner glow puddle — a blurred accent circle, low opacity.
          Positioned(
            bottom: -32,
            right: -32,
            child: ImageFiltered(
              imageFilter: ImageFilter.blur(sigmaX: 24, sigmaY: 24),
              child: Container(
                width: 96,
                height: 96,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: accent.withValues(alpha: 0.25),
                ),
              ),
            ),
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(title.toUpperCase(), style: RubixText.label),
              Text.rich(
                TextSpan(
                  text: '$value',
                  style: TextStyle(
                    fontSize: 30,
                    height: 1,
                    fontWeight: FontWeight.w800,
                    color: look.ink,
                  ),
                  children: [
                    if (unit != null)
                      TextSpan(
                        text: ' $unit',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                          color: look.inkMuted,
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
