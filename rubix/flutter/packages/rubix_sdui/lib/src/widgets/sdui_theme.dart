/// Visual tokens for the SDUI Flutter renderer.
///
/// Mirrors the React renderer's CSS-var contract documented in
/// `rubix/docs/design/sdui/visual-design-spec.md`. Host apps wire the
/// concrete colors by registering a populated [SduiTheme] on
/// `ThemeData.extensions`; the package ships sensible defaults
/// (teal-leaning) so it looks right out-of-the-box even without host
/// wiring.
library;

import 'package:flutter/material.dart';

/// Semantic accent role — five-way rotation for KPI tiles and chart
/// series. Keep in sync with the React `SduiAccent` union in
/// `packages/starter-ui-sdui-react/src/renderer/accent.ts`.
enum SduiAccent { leaf, aqua, sun, sky, warn }

/// Returns the concrete [Color] for [accent] resolved through the
/// active [SduiTheme]. Falls back to package defaults when no
/// extension is registered on the theme.
Color accentColor(BuildContext context, SduiAccent accent) {
  final t = SduiTheme.of(context);
  switch (accent) {
    case SduiAccent.leaf: return t.accentLeaf;
    case SduiAccent.aqua: return t.accentAqua;
    case SduiAccent.sun:  return t.accentSun;
    case SduiAccent.sky:  return t.accentSky;
    case SduiAccent.warn: return t.accentWarn;
  }
}

@immutable
class SduiTheme extends ThemeExtension<SduiTheme> {
  const SduiTheme({
    required this.accentLeaf,
    required this.accentAqua,
    required this.accentSun,
    required this.accentSky,
    required this.accentWarn,
    required this.statusOk,
    required this.statusDanger,
    required this.glassFill,
    required this.glassBorder,
    required this.hairline,
    required this.subtleText,
    required this.mutedText,
  });

  final Color accentLeaf;
  final Color accentAqua;
  final Color accentSun;
  final Color accentSky;
  final Color accentWarn;
  final Color statusOk;
  final Color statusDanger;

  /// Card surface fill behind glass blur.
  final Color glassFill;
  /// 1px outline around glass card.
  final Color glassBorder;
  /// Top-edge hairline color (typically derived from the card's accent).
  final Color hairline;
  /// KPI label color.
  final Color subtleText;
  /// Unit / trend / supporting copy color.
  final Color mutedText;

  /// Defaults tuned for a light surface (rubix `nube` palette).
  static const light = SduiTheme(
    accentLeaf:   Color(0xFF4497A2), // teal500
    accentAqua:   Color(0xFF0EA5B7), // cyan-teal
    accentSun:    Color(0xFFF5A314), // yellow500
    accentSky:    Color(0xFF3C83F6), // blue500
    accentWarn:   Color(0xFFF59F0A), // amber500
    statusOk:     Color(0xFF21C45D), // green500
    statusDanger: Color(0xFFEF4343), // red500
    glassFill:    Color(0xFFFFFFFF),
    glassBorder:  Color(0x14000000),
    hairline:     Color(0x33000000),
    subtleText:   Color(0xFFA3A3A3), // grey400
    mutedText:    Color(0xFF737373), // grey500
  );

  /// Defaults tuned for a dark surface.
  static const dark = SduiTheme(
    accentLeaf:   Color(0xFF61A3AE), // teal400, lifted for dark bg
    accentAqua:   Color(0xFF67E8F9), // cyan-300
    accentSun:    Color(0xFFFBBD41), // yellow400
    accentSky:    Color(0xFF7DD3FC), // sky-300
    accentWarn:   Color(0xFFFDE68A), // amber-200
    statusOk:     Color(0xFF22C55E),
    statusDanger: Color(0xFFFB7185),
    glassFill:    Color(0xCC171717), // grey900 @ 80%
    glassBorder:  Color(0x1FFFFFFF),
    hairline:     Color(0x33FFFFFF),
    subtleText:   Color(0xFF737373),
    mutedText:    Color(0xFFA3A3A3),
  );

  static SduiTheme of(BuildContext context) {
    return Theme.of(context).extension<SduiTheme>() ??
        (Theme.of(context).brightness == Brightness.dark ? dark : light);
  }

  @override
  SduiTheme copyWith({
    Color? accentLeaf, Color? accentAqua, Color? accentSun, Color? accentSky,
    Color? accentWarn, Color? statusOk, Color? statusDanger,
    Color? glassFill, Color? glassBorder, Color? hairline,
    Color? subtleText, Color? mutedText,
  }) {
    return SduiTheme(
      accentLeaf: accentLeaf ?? this.accentLeaf,
      accentAqua: accentAqua ?? this.accentAqua,
      accentSun: accentSun ?? this.accentSun,
      accentSky: accentSky ?? this.accentSky,
      accentWarn: accentWarn ?? this.accentWarn,
      statusOk: statusOk ?? this.statusOk,
      statusDanger: statusDanger ?? this.statusDanger,
      glassFill: glassFill ?? this.glassFill,
      glassBorder: glassBorder ?? this.glassBorder,
      hairline: hairline ?? this.hairline,
      subtleText: subtleText ?? this.subtleText,
      mutedText: mutedText ?? this.mutedText,
    );
  }

  @override
  SduiTheme lerp(ThemeExtension<SduiTheme>? other, double t) {
    if (other is! SduiTheme) return this;
    return SduiTheme(
      accentLeaf: Color.lerp(accentLeaf, other.accentLeaf, t)!,
      accentAqua: Color.lerp(accentAqua, other.accentAqua, t)!,
      accentSun: Color.lerp(accentSun, other.accentSun, t)!,
      accentSky: Color.lerp(accentSky, other.accentSky, t)!,
      accentWarn: Color.lerp(accentWarn, other.accentWarn, t)!,
      statusOk: Color.lerp(statusOk, other.statusOk, t)!,
      statusDanger: Color.lerp(statusDanger, other.statusDanger, t)!,
      glassFill: Color.lerp(glassFill, other.glassFill, t)!,
      glassBorder: Color.lerp(glassBorder, other.glassBorder, t)!,
      hairline: Color.lerp(hairline, other.hairline, t)!,
      subtleText: Color.lerp(subtleText, other.subtleText, t)!,
      mutedText: Color.lerp(mutedText, other.mutedText, t)!,
    );
  }
}
