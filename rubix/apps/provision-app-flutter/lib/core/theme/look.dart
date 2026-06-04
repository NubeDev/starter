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
    required this.accentOn,
    required this.accent2On,
    required this.ink,
    required this.inkSoft,
    required this.inkMuted,
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

  /// foreground on top of [accent] / [accent2]
  final Color accentOn;
  final Color accent2On;

  /// headline / body / muted text tints
  final Color ink;
  final Color inkSoft;
  final Color inkMuted;
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
      accentOn: theme.accentOn,
      accent2On: theme.accent2On,
      ink: theme.ink,
      inkSoft: theme.inkSoft,
      inkMuted: theme.inkMuted,
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

/// Exposes the resolved [Look] to the whole route subtree via the element tree,
/// so any widget — including plain [StatelessWidget]s with no [WidgetRef] — can
/// read it as `context.look` and rebuild when the theme or status changes.
/// Pushed once at the app root (see `app.dart`) from `lookProvider`.
class LookScope extends InheritedWidget {
  const LookScope({required this.look, required super.child, super.key});

  final Look look;

  static Look of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<LookScope>();
    assert(scope != null, 'No LookScope found in context');
    return scope!.look;
  }

  @override
  bool updateShouldNotify(LookScope oldWidget) => look != oldWidget.look;
}

/// Terse accessors so feature code reads `context.look` (or the per-color
/// shortcuts `context.accent`, `context.ink`, …) instead of threading a
/// provider. Backed by [LookScope].
///
/// Each shortcut resolves the [Look] once, so a single `context.accent` costs
/// exactly one inherited-widget lookup — identical to reading `look.accent`
/// off a hoisted local. When a single build touches several colors, still
/// prefer `final look = context.look;` and read fields off it, so the lookup
/// happens once rather than once per shortcut.
extension LookContext on BuildContext {
  Look get look => LookScope.of(this);

  // live accents + their on-colors
  Color get accent => look.accent;
  Color get accent2 => look.accent2;
  Color get accentOn => look.accentOn;
  Color get accent2On => look.accent2On;

  // text ramp
  Color get ink => look.ink;
  Color get inkSoft => look.inkSoft;
  Color get inkMuted => look.inkMuted;

  // page base + shape personality
  Color get base => look.base;
  double get lookRadius => look.radius;
}
