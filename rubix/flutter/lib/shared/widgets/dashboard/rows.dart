import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

// ---------------------------------------------------------------------------
// NubeActivityRow — icon tile + title + subtitle + trailing timestamp.
// Used inside the LIVING SIGNAL card on the Live Dashboard.
// ---------------------------------------------------------------------------
class NubeActivityRow extends StatelessWidget {
  const NubeActivityRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.timestamp,
    this.tone = NubeGlowTone.teal,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final String timestamp;
  final NubeGlowTone tone;

  Color _iconColor(BuildContext context) {
    final t = Theme.of(context).nube;
    switch (tone) {
      case NubeGlowTone.green:
        return t.success;
      case NubeGlowTone.amber:
        return t.warning;
      case NubeGlowTone.danger:
        return t.danger;
      case NubeGlowTone.none:
      case NubeGlowTone.teal:
        return t.leaf;
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final tint = _iconColor(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: tint.withValues(alpha: 0.10),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: tint.withValues(alpha: 0.20)),
            ),
            alignment: Alignment.center,
            child: Icon(icon, size: 16, color: tint),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 13.5,
                    fontWeight: FontWeight.w600,
                    letterSpacing: -0.1,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 2),
                Text(
                  subtitle,
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 12,
                    fontWeight: FontWeight.w400,
                    height: 1.2,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Text(
            timestamp,
            style: TextStyle(
              color: t.subtle,
              fontSize: 11.5,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// NubeProgressRow — site row: leading icon + label + slim themed bar +
// trailing percentage. Used inside the SITE BY SITE card.
// ---------------------------------------------------------------------------
class NubeProgressRow extends StatelessWidget {
  const NubeProgressRow({
    required this.icon,
    required this.label,
    required this.percent,
    this.tone = NubeGlowTone.teal,
    super.key,
  });

  final IconData icon;
  final String label;

  /// 0..100.
  final double percent;
  final NubeGlowTone tone;

  Color _barColor(BuildContext context) {
    final t = Theme.of(context).nube;
    switch (tone) {
      case NubeGlowTone.green:
        return t.success;
      case NubeGlowTone.amber:
        return t.warning;
      case NubeGlowTone.danger:
        return t.danger;
      case NubeGlowTone.none:
      case NubeGlowTone.teal:
        return t.leaf;
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final bar = _barColor(context);
    final pct = percent.clamp(0.0, 100.0);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 14, color: t.muted),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  label,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 13,
                    fontWeight: FontWeight.w500,
                    letterSpacing: -0.1,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text(
                '${pct.toStringAsFixed(0)}%',
                style: TextStyle(
                  color: t.text,
                  fontSize: 12.5,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(999),
            child: TweenAnimationBuilder<double>(
              tween: Tween(begin: 0, end: pct / 100),
              duration: const Duration(milliseconds: 700),
              curve: Curves.easeOutCubic,
              builder: (context, v, _) => LinearProgressIndicator(
                value: v,
                minHeight: 4,
                backgroundColor: t.surface2,
                valueColor: AlwaysStoppedAnimation<Color>(bar),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
