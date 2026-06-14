import 'package:flutter/material.dart';

import 'package:provision_app/core/theme/app_themes.dart';
import 'package:provision_app/core/theme/statuses.dart';

/// The RESOLVED look every widget reads. Merges two layers, exactly like the
/// React app's `useLook()`:
///   1. app theme   → base palette, shape personality (persistent)
///   2. live status → tints the accent on top (transient: online/pairing/…)
/// Widgets never merge these themselves; they read [Look] from the `look`
/// provider (or, for the ambient theme data, via [BuildContext.look]).
@immutable
class Look {
  const Look({
    required this.accent,
    required this.accent2,
    required this.ink,
    required this.inkSoft,
    required this.base,
    required this.radius,
    required this.glowAlpha,
    required this.baseGradient,
    required this.statusTint,
    required this.themeAccent,
    required this.statusAccent,
  });

  /// live accent — status wins if set, else theme accent
  final Color accent;
  final Color accent2;
  final Color ink;
  final Color inkSoft;
  final Color base;
  final double radius;
  final double glowAlpha;
  final List<RadialWash> baseGradient;

  /// an optional extra wash contributed by the active status (null if none)
  final RadialWash? statusTint;
  final Color themeAccent;
  final Color? statusAccent;

  /// Resolve the look from the chosen [theme] and an optional live [status].
  factory Look.resolve(AppTheme theme, StatusKey? status) {
    final statusColor = status != null ? statuses[status]!.accent : null;
    final accent = statusColor ?? theme.accent;
    return Look(
      accent: accent,
      accent2: theme.accent2,
      ink: theme.ink,
      inkSoft: theme.inkSoft,
      base: theme.base,
      radius: theme.radius,
      glowAlpha: theme.glowAlpha,
      baseGradient: theme.gradient,
      statusTint: statusColor != null
          ? RadialWash(
              color: statusColor.withValues(alpha: 0.1),
              alignment: Alignment.center,
              radius: 1.2,
            )
          : null,
      themeAccent: theme.accent,
      statusAccent: statusColor,
    );
  }

  /// A stable key for cross-fading the backdrop when theme/status changes.
  String get backdropKey =>
      '${themeAccent.toARGB32()}|${statusAccent?.toARGB32() ?? 'none'}';
}
