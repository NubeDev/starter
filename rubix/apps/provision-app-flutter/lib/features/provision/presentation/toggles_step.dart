import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// Trending / Alarming switches, pre-filled from template defaults upstream.
/// Ported from the React `TogglesStep.tsx`.
class TogglesStep extends ConsumerWidget {
  const TogglesStep({
    required this.trend,
    required this.alarm,
    required this.onTrend,
    required this.onAlarm,
    super.key,
  });

  final bool trend;
  final bool alarm;
  final ValueChanged<bool> onTrend;
  final ValueChanged<bool> onAlarm;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = ref.watch(lookProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Row(
          icon: LucideIcons.activity,
          title: 'Trending',
          blurb: 'Record point history for charts',
          on: trend,
          onToggle: () => onTrend(!trend),
          accent: look.accent,
        ),
        const SizedBox(height: 12),
        _Row(
          icon: LucideIcons.bellRing,
          title: 'Alarming',
          blurb: 'Arm template alarm thresholds',
          on: alarm,
          onToggle: () => onAlarm(!alarm),
          accent: look.accent,
        ),
      ],
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.icon,
    required this.title,
    required this.blurb,
    required this.on,
    required this.onToggle,
    required this.accent,
  });

  final IconData icon;
  final String title;
  final String blurb;
  final bool on;
  final VoidCallback onToggle;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final look = context.look;
    return GlassSurface(
      borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
      padding: const EdgeInsets.all(16),
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: accent.withValues(alpha: 0.13),
              borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            ),
            child: Icon(icon, size: 20, color: accent),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: TextStyle(
                    color: look.ink,
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  blurb,
                  style: TextStyle(
                    color: look.inkMuted,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          GlassToggle(
            on: on,
            onToggle: onToggle,
            accent: accent,
            semanticLabel: title,
          ),
        ],
      ),
    );
  }
}
