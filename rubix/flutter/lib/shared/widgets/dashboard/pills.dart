import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

// ---------------------------------------------------------------------------
// NubeStatusPill — small uppercase tracked badge with a leading dot.
// e.g. `● LIVE · 412 DEVICES STREAMING`.
// ---------------------------------------------------------------------------
class NubeStatusPill extends StatelessWidget {
  const NubeStatusPill({
    required this.label,
    this.tone = NubeGlowTone.teal,
    this.pulse = true,
    super.key,
  });

  final String label;
  final NubeGlowTone tone;
  final bool pulse;

  Color _dotColor(BuildContext context) {
    final t = Theme.of(context).nube;
    switch (tone) {
      case NubeGlowTone.none:
      case NubeGlowTone.teal:
        return t.leaf;
      case NubeGlowTone.green:
        return t.success;
      case NubeGlowTone.amber:
        return t.warning;
      case NubeGlowTone.danger:
        return t.danger;
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final dot = _dotColor(context);
    Widget dotWidget = Container(
      width: 6,
      height: 6,
      decoration: BoxDecoration(
        color: dot,
        shape: BoxShape.circle,
        boxShadow: [
          BoxShadow(
            color: dot.withValues(alpha: 0.6),
            blurRadius: 6,
            spreadRadius: -1,
          ),
        ],
      ),
    );
    if (pulse) {
      dotWidget = dotWidget
          .animate(onPlay: (c) => c.repeat(reverse: true))
          .fadeOut(
            duration: 900.ms,
            curve: Curves.easeInOut,
            begin: 0.35,
          );
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: dot.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: dot.withValues(alpha: 0.35)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          dotWidget,
          const SizedBox(width: 8),
          Text(
            label.toUpperCase(),
            style: TextStyle(
              color: t.text,
              fontSize: 10.5,
              fontWeight: FontWeight.w600,
              letterSpacing: 1.2,
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeEyebrow — tiny horizontal teal rule + tracked uppercase label.
// `— LIVE DASHBOARD`.
// ---------------------------------------------------------------------------
class NubeEyebrow extends StatelessWidget {
  const NubeEyebrow(this.label, {this.tone = NubeGlowTone.teal, super.key});

  final String label;
  final NubeGlowTone tone;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final Color color;
    switch (tone) {
      case NubeGlowTone.green:
        color = t.success;
      case NubeGlowTone.amber:
        color = t.warning;
      case NubeGlowTone.danger:
        color = t.danger;
      case NubeGlowTone.none:
      case NubeGlowTone.teal:
        color = t.leaf;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(width: 22, height: 2, color: color),
        const SizedBox(width: 10),
        Text(
          label.toUpperCase(),
          style: TextStyle(
            color: t.muted,
            fontSize: 10.5,
            fontWeight: FontWeight.w600,
            letterSpacing: 1.4,
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// NubeKbd — small bordered keyboard hint chip. `⌘K`, `G H`, …
// ---------------------------------------------------------------------------
class NubeKbd extends StatelessWidget {
  const NubeKbd(this.label, {super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        color: t.surface2,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: t.border),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: t.muted,
          fontSize: 10,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.4,
          height: 1.4,
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeTrendChip — `↑ 2.4%` (green) / `↓ 1.2%` (red).
// ---------------------------------------------------------------------------
class NubeTrendChip extends StatelessWidget {
  const NubeTrendChip({required this.delta, super.key});

  /// Positive value renders as success/up; negative as danger/down.
  final double delta;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final up = delta >= 0;
    final color = up ? t.success : t.danger;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withValues(alpha: 0.25)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            up ? LucideIcons.arrowUp : LucideIcons.arrowDown,
            size: 10,
            color: color,
          ),
          const SizedBox(width: 2),
          Text(
            '${delta.abs().toStringAsFixed(1)}%',
            style: TextStyle(
              color: color,
              fontSize: 10.5,
              fontWeight: FontWeight.w600,
              letterSpacing: 0.2,
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeSegmented — small bordered segmented control. Picks a value from
// [options]. Wraps Material's [SegmentedButton] which is already themed
// in app_theme.dart with the shadcn flavour.
// ---------------------------------------------------------------------------
class NubeSegmented<T> extends StatelessWidget {
  const NubeSegmented({
    required this.options,
    required this.value,
    required this.onChanged,
    super.key,
  });

  final List<({T value, String label})> options;
  final T value;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<T>(
      showSelectedIcon: false,
      segments: [
        for (final o in options)
          ButtonSegment<T>(value: o.value, label: Text(o.label)),
      ],
      selected: {value},
      onSelectionChanged: (s) => onChanged(s.first),
    );
  }
}
