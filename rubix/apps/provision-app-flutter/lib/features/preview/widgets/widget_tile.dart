import 'package:flutter/material.dart';
import 'package:provision_app/features/preview/widgets/battery_widget.dart';
import 'package:provision_app/features/preview/widgets/counter_widget.dart';
import 'package:provision_app/features/preview/widgets/gauge_widget.dart';
import 'package:provision_app/features/preview/widgets/led_widget.dart';
import 'package:provision_app/features/preview/widgets/line_widget.dart';
import 'package:provision_app/features/preview/widgets/stat_widget.dart';
import 'package:provision_app/features/preview/widgets/toggle_widget.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';

/// The renderer switchboard: maps a widget enum → its component, inside a glass
/// tile. `bc_widgets` rows say "render `gauge` for point X"; this mounts it.
/// Demo values are deterministic from `seed` (no live ingest yet). Ported from
/// the React `WidgetTile`.
class WidgetTile extends StatelessWidget {
  const WidgetTile({
    required this.widget,
    required this.title,
    required this.accent,
    this.unit,
    this.seed = 0,
    super.key,
  });

  final String widget;
  final String title;
  final String? unit;
  final Color accent;
  final int seed;

  @override
  Widget build(BuildContext context) {
    // Stable per-tile demo reading derived from the seed.
    final demo = 20 + ((seed * 37) % 70);
    return GlassCard(
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 128),
        child: _render(demo),
      ),
    );
  }

  Widget _render(int demo) {
    switch (widget) {
      case 'gauge':
        return GaugeWidget(
          title: title,
          unit: unit,
          value: demo.toDouble(),
          accent: accent,
        );
      case 'battery':
        return BatteryWidget(
          title: title,
          value: demo.toDouble(),
          accent: accent,
        );
      case 'counter':
        return CounterWidget(
          title: title,
          unit: unit,
          value: demo * 128,
          accent: accent,
        );
      case 'led':
        return LedWidget(title: title, on: demo > 45, accent: accent);
      case 'toggle':
        return ToggleWidget(title: title, on: demo > 45, accent: accent);
      case 'line':
        return LineWidget(
          title: title,
          unit: unit,
          accent: accent,
          seed: seed,
        );
      case 'stat':
      default:
        return StatWidget(
          title: title,
          unit: unit,
          value: demo,
          accent: accent,
        );
    }
  }
}
