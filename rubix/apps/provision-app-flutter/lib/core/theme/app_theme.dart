import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

import 'package:provision_app/core/theme/tokens.dart';

export 'package:provision_app/core/theme/tokens.dart';

/// The Material [ThemeData] for the app. The visual identity lives in the glass
/// widgets + the `look` provider (ported from the React Tailwind layer); this
/// only provides the dark baseline, the Plus Jakarta Sans text theme, and a
/// thorough "de-Materialization" (no ripples, no surface tints, no elevation
/// overlays) so nothing reads as a stock Google app — same intent as the
/// sibling Flutter project's `_build`.
ThemeData buildRubixTheme() {
  const scheme = ColorScheme.dark(
    primary: RubixTokens.primary,
    onPrimary: RubixTokens.primaryOn,
    primaryContainer: RubixTokens.primaryContainer,
    secondary: RubixTokens.coral,
    onSecondary: RubixTokens.coralOn,
    surface: RubixTokens.surface,
    onSurface: RubixTokens.ink,
    onSurfaceVariant: RubixTokens.inkVariant,
    error: RubixTokens.fault,
    outline: RubixTokens.inkMuted,
  );

  final baseText = GoogleFonts.plusJakartaSansTextTheme(
    ThemeData(brightness: Brightness.dark).textTheme,
  ).apply(bodyColor: RubixTokens.ink, displayColor: RubixTokens.ink);

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: RubixTokens.obsidian,
    canvasColor: RubixTokens.obsidian,
    textTheme: baseText,
    primaryTextTheme: baseText,
    // Kill the Material ink ripple everywhere — the glass widgets drive their
    // own scale-on-tap feedback instead. Biggest single "de-Material" lever.
    splashFactory: NoSplash.splashFactory,
    splashColor: Colors.transparent,
    highlightColor: Colors.transparent,
    hoverColor: Colors.white.withValues(alpha: 0.04),
    applyElevationOverlayColor: false,
    pageTransitionsTheme: const PageTransitionsTheme(
      builders: {
        TargetPlatform.android: FadeForwardsPageTransitionsBuilder(),
        TargetPlatform.iOS: CupertinoPageTransitionsBuilder(),
        TargetPlatform.macOS: CupertinoPageTransitionsBuilder(),
        TargetPlatform.linux: FadeForwardsPageTransitionsBuilder(),
        TargetPlatform.windows: FadeForwardsPageTransitionsBuilder(),
      },
    ),
    dialogTheme: const DialogThemeData(
      backgroundColor: RubixTokens.surfaceLow,
      surfaceTintColor: Colors.transparent,
    ),
    bottomSheetTheme: const BottomSheetThemeData(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      modalBackgroundColor: Colors.transparent,
      elevation: 0,
    ),
  );
}

/// Text styles that map the React `@theme` type ramp (headline-mobile,
/// label-sm, etc.) onto Flutter [TextStyle]s. Read via [BuildContext]
/// extensions in the widgets so feature code stays terse.
abstract final class RubixText {
  static const headlineMobile = TextStyle(
    fontSize: 28,
    height: 34 / 28,
    fontWeight: FontWeight.w700,
    color: RubixTokens.ink,
  );

  static const headlineLg = TextStyle(
    fontSize: 32,
    height: 40 / 32,
    letterSpacing: -0.32,
    fontWeight: FontWeight.w700,
    color: RubixTokens.ink,
  );

  /// Uppercase architectural label — the React `.label` utility
  /// (`text-label-sm uppercase text-ink-muted`).
  static const label = TextStyle(
    fontSize: 12,
    height: 16 / 12,
    letterSpacing: 0.6,
    fontWeight: FontWeight.w600,
    color: RubixTokens.inkMuted,
  );
}
