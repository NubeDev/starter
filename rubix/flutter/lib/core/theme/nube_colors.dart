import 'package:flutter/material.dart';

/// Nube iO design system — raw color ramps.
///
/// Mirrors `rubix/frontend/src/styles/primitives.css`. Do NOT reference these
/// from feature code — always go through the semantic [NubeTokens] layer
/// below, exposed through [ThemeData.colorScheme] and [ThemeExtension]s.
///
/// Source of truth: `rubix/frontend/src/styles/primitives.css`.
abstract final class NubePrimitives {
  // Teal — brand
  static const teal50  = Color(0xFFF2F7F7);
  static const teal100 = Color(0xFFD8E7E9);
  static const teal200 = Color(0xFFB3D1D6);
  static const teal300 = Color(0xFF83B6BE);
  static const teal400 = Color(0xFF61A3AE);
  static const teal500 = Color(0xFF4497A2); // ⭐ brand
  static const teal600 = Color(0xFF36828C);
  static const teal700 = Color(0xFF2D6971);
  static const teal800 = Color(0xFF295A61);
  static const teal900 = Color(0xFF234D52);
  static const teal950 = Color(0xFF142F33);

  // Yellow — callout (underlines + small badges only)
  static const yellow50  = Color(0xFFFFFAEB);
  static const yellow100 = Color(0xFFFFF2C7);
  static const yellow200 = Color(0xFFFFE48A);
  static const yellow300 = Color(0xFFFFD24D);
  static const yellow400 = Color(0xFFFBBD41); // ⭐ callout
  static const yellow500 = Color(0xFFF5A314);
  static const yellow600 = Color(0xFFDC7F04);
  static const yellow700 = Color(0xFFB45908);
  static const yellow800 = Color(0xFF95460E);
  static const yellow900 = Color(0xFF76360F);

  // Grey — neutral
  static const grey50  = Color(0xFFFAFAFA);
  static const grey100 = Color(0xFFF5F5F5);
  static const grey200 = Color(0xFFE6E6E6);
  static const grey300 = Color(0xFFD4D4D4);
  static const grey400 = Color(0xFFA3A3A3);
  static const grey500 = Color(0xFF737373);
  static const grey600 = Color(0xFF525252);
  static const grey700 = Color(0xFF404040);
  static const grey800 = Color(0xFF262626);
  static const grey900 = Color(0xFF171717);
  static const grey950 = Color(0xFF0A0A0A);

  // Status
  static const green500 = Color(0xFF21C45D);
  static const amber500 = Color(0xFFF59F0A);
  static const red500   = Color(0xFFEF4343);
  static const blue500  = Color(0xFF3C83F6);
}

/// Semantic tokens exposed as a [ThemeExtension]. Mirrors the
/// `--tk-*` bridge layer in `rubix/frontend/src/styles/theme.css` so
/// feature widgets have access to Nube-specific surfaces (sidebar,
/// muted, callout, etc.) that don't fit cleanly in [ColorScheme].
@immutable
class NubeTokens extends ThemeExtension<NubeTokens> {
  const NubeTokens({
    required this.bg,
    required this.bg2,
    required this.surface,
    required this.surface2,
    required this.border,
    required this.borderHi,
    required this.text,
    required this.muted,
    required this.subtle,
    required this.leaf,
    required this.leaf2,
    required this.callout,
    required this.calloutForeground,
    required this.success,
    required this.warning,
    required this.danger,
    required this.info,
  });

  final Color bg;
  final Color bg2;
  final Color surface;
  final Color surface2;
  final Color border;
  final Color borderHi;
  final Color text;
  final Color muted;
  final Color subtle;
  final Color leaf;
  final Color leaf2;
  final Color callout;
  final Color calloutForeground;
  final Color success;
  final Color warning;
  final Color danger;
  final Color info;

  static const light = NubeTokens(
    bg:        NubePrimitives.grey50,
    bg2:       NubePrimitives.grey100,
    surface:   Color(0xFFFFFFFF),
    surface2:  NubePrimitives.grey100,
    border:    NubePrimitives.grey200,
    borderHi:  NubePrimitives.teal200,
    text:      NubePrimitives.grey800,
    muted:     NubePrimitives.grey500,
    subtle:    NubePrimitives.grey400,
    leaf:      NubePrimitives.teal500,
    leaf2:     NubePrimitives.teal600,
    callout:   NubePrimitives.yellow400,
    calloutForeground: NubePrimitives.grey800,
    success:   NubePrimitives.green500,
    warning:   NubePrimitives.amber500,
    danger:    NubePrimitives.red500,
    info:      NubePrimitives.blue500,
  );

  static const dark = NubeTokens(
    bg:        NubePrimitives.grey950,
    bg2:       NubePrimitives.grey900,
    surface:   NubePrimitives.grey900,
    surface2:  NubePrimitives.grey800,
    border:    NubePrimitives.grey800,
    borderHi:  NubePrimitives.teal800,
    text:      Color(0xFFF5F5F5),
    muted:     NubePrimitives.grey400,
    subtle:    NubePrimitives.grey500,
    leaf:      NubePrimitives.teal400, // lifted for dark bg
    leaf2:     NubePrimitives.teal300,
    callout:   NubePrimitives.yellow400,
    calloutForeground: NubePrimitives.grey950,
    success:   NubePrimitives.green500,
    warning:   NubePrimitives.amber500,
    danger:    NubePrimitives.red500,
    info:      NubePrimitives.blue500,
  );

  @override
  NubeTokens copyWith({
    Color? bg, Color? bg2, Color? surface, Color? surface2,
    Color? border, Color? borderHi, Color? text, Color? muted, Color? subtle,
    Color? leaf, Color? leaf2, Color? callout, Color? calloutForeground,
    Color? success, Color? warning, Color? danger, Color? info,
  }) {
    return NubeTokens(
      bg: bg ?? this.bg,
      bg2: bg2 ?? this.bg2,
      surface: surface ?? this.surface,
      surface2: surface2 ?? this.surface2,
      border: border ?? this.border,
      borderHi: borderHi ?? this.borderHi,
      text: text ?? this.text,
      muted: muted ?? this.muted,
      subtle: subtle ?? this.subtle,
      leaf: leaf ?? this.leaf,
      leaf2: leaf2 ?? this.leaf2,
      callout: callout ?? this.callout,
      calloutForeground: calloutForeground ?? this.calloutForeground,
      success: success ?? this.success,
      warning: warning ?? this.warning,
      danger: danger ?? this.danger,
      info: info ?? this.info,
    );
  }

  @override
  NubeTokens lerp(ThemeExtension<NubeTokens>? other, double t) {
    if (other is! NubeTokens) return this;
    return NubeTokens(
      bg: Color.lerp(bg, other.bg, t)!,
      bg2: Color.lerp(bg2, other.bg2, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      surface2: Color.lerp(surface2, other.surface2, t)!,
      border: Color.lerp(border, other.border, t)!,
      borderHi: Color.lerp(borderHi, other.borderHi, t)!,
      text: Color.lerp(text, other.text, t)!,
      muted: Color.lerp(muted, other.muted, t)!,
      subtle: Color.lerp(subtle, other.subtle, t)!,
      leaf: Color.lerp(leaf, other.leaf, t)!,
      leaf2: Color.lerp(leaf2, other.leaf2, t)!,
      callout: Color.lerp(callout, other.callout, t)!,
      calloutForeground: Color.lerp(calloutForeground, other.calloutForeground, t)!,
      success: Color.lerp(success, other.success, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
      info: Color.lerp(info, other.info, t)!,
    );
  }
}

/// Convenience accessor: `Theme.of(context).nube`.
extension NubeThemeX on ThemeData {
  NubeTokens get nube => extension<NubeTokens>() ?? NubeTokens.light;
}
