import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:rubix_flutter/core/theme/nube_colors.dart';

export 'package:rubix_flutter/core/theme/nube_colors.dart';

/// Rubix app theme — ports the Nube iO design system from the React app
/// (`rubix/frontend/src/styles/{primitives,tokens,theme}.css`) to Flutter.
///
/// Brand color: teal-500 (#4497A2). Neutral ramp: pure greys. Status colors
/// match the React semantic layer. Yellow is reserved for callouts.
ThemeData get rubixLightTheme => _build(Brightness.light, NubeTokens.light);
ThemeData get rubixDarkTheme  => _build(Brightness.dark,  NubeTokens.dark);

/// Italic-serif accent style — used inline inside [Text.rich] for display
/// accents like "operating layer for the *physical world.*". Pairs with
/// Inter at the same font size; matches the mockups (Instrument Serif).
TextStyle accentItalicTextStyle(BuildContext context, {double? fontSize}) {
  final base = DefaultTextStyle.of(context).style;
  return GoogleFonts.instrumentSerif(
    fontStyle: FontStyle.italic,
    fontWeight: FontWeight.w400,
    fontSize: fontSize ?? base.fontSize,
    color: base.color,
    height: base.height,
    letterSpacing: -0.5,
  );
}

ThemeData _build(Brightness brightness, NubeTokens t) {
  final isDark = brightness == Brightness.dark;

  final scheme = ColorScheme(
    brightness: brightness,
    primary: t.leaf,
    onPrimary: isDark ? NubePrimitives.grey950 : Colors.white,
    primaryContainer: isDark ? NubePrimitives.teal950 : NubePrimitives.teal50,
    onPrimaryContainer: isDark ? NubePrimitives.teal300 : NubePrimitives.teal700,
    secondary: t.surface2,
    onSecondary: t.text,
    secondaryContainer: t.surface2,
    onSecondaryContainer: t.text,
    tertiary: t.callout,
    onTertiary: t.calloutForeground,
    tertiaryContainer: isDark ? NubePrimitives.yellow900 : NubePrimitives.yellow100,
    onTertiaryContainer: isDark ? NubePrimitives.yellow200 : NubePrimitives.yellow800,
    error: t.danger,
    onError: Colors.white,
    errorContainer: isDark ? const Color(0xFF4A1414) : const Color(0xFFFFE4E4),
    onErrorContainer: isDark ? const Color(0xFFFFD4D4) : const Color(0xFF7A1010),
    surface: t.surface,
    onSurface: t.text,
    surfaceContainerLowest: t.bg,
    surfaceContainerLow:    t.bg2,
    surfaceContainer:       t.surface,
    surfaceContainerHigh:   t.surface2,
    surfaceContainerHighest: t.surface2,
    onSurfaceVariant: t.muted,
    outline: t.border,
    outlineVariant: t.border,
    shadow: Colors.black,
    scrim: Colors.black,
    inverseSurface: isDark ? NubePrimitives.grey50 : NubePrimitives.grey900,
    onInverseSurface: isDark ? NubePrimitives.grey900 : NubePrimitives.grey50,
    inversePrimary: isDark ? NubePrimitives.teal700 : NubePrimitives.teal300,
  );

  // Font: Inter — the React app's `--font-sans` stack lists Geist first, but
  // Geist isn't published on Google Fonts. Inter is the next entry and is
  // visually near-identical (geometric humanist sans, same x-height bracket).
  final baseText = GoogleFonts.interTextTheme(
    ThemeData(brightness: brightness).textTheme,
  ).apply(bodyColor: t.text, displayColor: t.text);

  final base = ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    scaffoldBackgroundColor: t.bg,
    canvasColor: t.bg,
    dividerColor: t.border,
    // Kill the Material ink ripple — replace with a flat hover/press tint
    // via WidgetState overlays on each component. This single change is the
    // biggest "de-Material" lever.
    splashFactory: NoSplash.splashFactory,
    splashColor: Colors.transparent,
    highlightColor: Colors.transparent,
    hoverColor: t.surface2,
    focusColor: t.leaf.withValues(alpha: 0.12),
    // M3 paints a translucent tint on top of every elevated surface tied to
    // colorScheme.surfaceTint. Disable globally — feels like a web app.
    applyElevationOverlayColor: false,
    // Smooth platform-correct page transitions instead of the bouncy
    // Material zoom.
    pageTransitionsTheme: const PageTransitionsTheme(builders: {
      TargetPlatform.android: FadeForwardsPageTransitionsBuilder(),
      TargetPlatform.iOS:     CupertinoPageTransitionsBuilder(),
      TargetPlatform.macOS:   CupertinoPageTransitionsBuilder(),
      TargetPlatform.linux:   FadeForwardsPageTransitionsBuilder(),
      TargetPlatform.windows: FadeForwardsPageTransitionsBuilder(),
      TargetPlatform.fuchsia: FadeForwardsPageTransitionsBuilder(),
    }),
    textTheme: baseText,
    primaryTextTheme: baseText,
    extensions: <ThemeExtension<dynamic>>[t],
  );

  final textTheme = baseText;

  return base.copyWith(
    textTheme: textTheme,
    primaryTextTheme: textTheme,
    appBarTheme: AppBarTheme(
      backgroundColor: t.bg,
      foregroundColor: t.text,
      surfaceTintColor: Colors.transparent,
      shadowColor: Colors.transparent,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleSpacing: 16,
      toolbarHeight: 56,
      titleTextStyle: textTheme.titleMedium?.copyWith(
        color: t.text,
        fontWeight: FontWeight.w600,
        letterSpacing: -0.1,
      ),
      iconTheme: IconThemeData(color: t.text, size: 20),
      actionsIconTheme: IconThemeData(color: t.muted, size: 20),
    ),
    cardTheme: CardThemeData(
      color: t.surface,
      surfaceTintColor: Colors.transparent,
      shadowColor: Colors.transparent,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: t.border),
      ),
    ),
    dividerTheme: DividerThemeData(color: t.border, thickness: 1, space: 1),
    iconTheme: IconThemeData(color: t.text),
    listTileTheme: ListTileThemeData(
      iconColor: t.muted,
      textColor: t.text,
      tileColor: Colors.transparent,
      selectedColor: t.leaf,
      selectedTileColor: isDark ? NubePrimitives.teal950 : NubePrimitives.teal50,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      minVerticalPadding: 8,
      horizontalTitleGap: 12,
      visualDensity: VisualDensity.standard,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
      indicatorColor: isDark ? NubePrimitives.teal950 : NubePrimitives.teal50,
      iconTheme: WidgetStatePropertyAll(IconThemeData(color: t.muted)),
      labelTextStyle: WidgetStatePropertyAll(
        textTheme.labelMedium?.copyWith(color: t.muted),
      ),
      elevation: 0,
      height: 64,
    ),
    navigationRailTheme: NavigationRailThemeData(
      backgroundColor: t.surface,
      indicatorColor: isDark ? NubePrimitives.teal950 : NubePrimitives.teal50,
      selectedIconTheme: IconThemeData(color: t.leaf),
      unselectedIconTheme: IconThemeData(color: t.muted),
      selectedLabelTextStyle: TextStyle(color: t.text, fontWeight: FontWeight.w600),
      unselectedLabelTextStyle: TextStyle(color: t.muted),
    ),
    drawerTheme: DrawerThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
      modalBackgroundColor: t.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: t.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: isDark ? NubePrimitives.grey100 : NubePrimitives.grey900,
      contentTextStyle: TextStyle(
        color: isDark ? NubePrimitives.grey900 : NubePrimitives.grey50,
      ),
      behavior: SnackBarBehavior.floating,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
    ),
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: isDark ? NubePrimitives.grey100 : NubePrimitives.grey900,
        borderRadius: BorderRadius.circular(6),
      ),
      textStyle: TextStyle(
        color: isDark ? NubePrimitives.grey900 : NubePrimitives.grey50,
        fontSize: 12,
      ),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: t.surface2,
      selectedColor: t.leaf,
      secondarySelectedColor: t.leaf,
      labelStyle: TextStyle(color: t.text),
      secondaryLabelStyle: TextStyle(color: isDark ? NubePrimitives.grey950 : Colors.white),
      side: BorderSide(color: t.border),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(999)),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: t.surface,
      hintStyle: TextStyle(color: t.subtle),
      labelStyle: TextStyle(color: t.muted),
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: t.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: t.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: t.leaf, width: 1.5),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: t.danger),
      ),
      focusedErrorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: t.danger, width: 1.5),
      ),
    ),
    // shadcn-flavoured SegmentedButton: small radius, no surface tint,
    // selected segment uses teal-tinted surface instead of filled primary.
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.selected)) {
            return isDark ? NubePrimitives.teal950 : NubePrimitives.teal50;
          }
          return t.surface;
        }),
        foregroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.selected)) return t.leaf;
          return t.muted;
        }),
        overlayColor: const WidgetStatePropertyAll(Colors.transparent),
        side: WidgetStatePropertyAll(BorderSide(color: t.border)),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        textStyle: const WidgetStatePropertyAll(
          TextStyle(fontWeight: FontWeight.w500, fontSize: 13),
        ),
        padding: const WidgetStatePropertyAll(
          EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        ),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.disabled)) return t.surface2;
          if (s.contains(WidgetState.hovered)) return t.leaf2;
          return t.leaf;
        }),
        foregroundColor: WidgetStatePropertyAll(
          isDark ? NubePrimitives.grey950 : Colors.white,
        ),
        elevation: const WidgetStatePropertyAll(0),
        shadowColor: const WidgetStatePropertyAll(Colors.transparent),
        overlayColor: const WidgetStatePropertyAll(Colors.transparent),
        padding: const WidgetStatePropertyAll(
          EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        textStyle: const WidgetStatePropertyAll(
          TextStyle(fontWeight: FontWeight.w500, fontSize: 14, letterSpacing: 0),
        ),
        minimumSize: const WidgetStatePropertyAll(Size(0, 40)),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.disabled)) return t.surface2;
          if (s.contains(WidgetState.hovered)) return t.leaf2;
          return t.leaf;
        }),
        foregroundColor: WidgetStatePropertyAll(
          isDark ? NubePrimitives.grey950 : Colors.white,
        ),
        overlayColor: const WidgetStatePropertyAll(Colors.transparent),
        elevation: const WidgetStatePropertyAll(0),
        shadowColor: const WidgetStatePropertyAll(Colors.transparent),
        padding: const WidgetStatePropertyAll(
          EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        textStyle: const WidgetStatePropertyAll(
          TextStyle(fontWeight: FontWeight.w500, fontSize: 14),
        ),
        minimumSize: const WidgetStatePropertyAll(Size(0, 40)),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.hovered)) return t.surface2;
          return Colors.transparent;
        }),
        foregroundColor: WidgetStatePropertyAll(t.text),
        side: WidgetStatePropertyAll(BorderSide(color: t.border)),
        overlayColor: const WidgetStatePropertyAll(Colors.transparent),
        padding: const WidgetStatePropertyAll(
          EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        ),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        textStyle: const WidgetStatePropertyAll(
          TextStyle(fontWeight: FontWeight.w500, fontSize: 14),
        ),
        minimumSize: const WidgetStatePropertyAll(Size(0, 40)),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: ButtonStyle(
        foregroundColor: WidgetStatePropertyAll(t.leaf),
        backgroundColor: WidgetStateProperty.resolveWith((s) {
          if (s.contains(WidgetState.hovered)) return t.surface2;
          return Colors.transparent;
        }),
        overlayColor: const WidgetStatePropertyAll(Colors.transparent),
        padding: const WidgetStatePropertyAll(
          EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        ),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        ),
        textStyle: const WidgetStatePropertyAll(
          TextStyle(fontWeight: FontWeight.w500, fontSize: 14),
        ),
        minimumSize: const WidgetStatePropertyAll(Size(0, 36)),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: t.leaf,
      foregroundColor: isDark ? NubePrimitives.grey950 : Colors.white,
      elevation: 0,
      focusElevation: 0,
      hoverElevation: 0,
      highlightElevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    // Slim shadcn-style switch (no oversized Material thumb).
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith(
        (s) => Colors.white,
      ),
      trackColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? t.leaf : t.surface2,
      ),
      trackOutlineColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? Colors.transparent : t.border,
      ),
      trackOutlineWidth: const WidgetStatePropertyAll(1),
      thumbIcon: const WidgetStatePropertyAll(
        Icon(Icons.circle, size: 0, color: Colors.transparent),
      ),
      splashRadius: 0,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? t.leaf : Colors.transparent,
      ),
      checkColor: WidgetStatePropertyAll(
        isDark ? NubePrimitives.grey950 : Colors.white,
      ),
      side: BorderSide(color: t.border, width: 1.5),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
      splashRadius: 0,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      overlayColor: const WidgetStatePropertyAll(Colors.transparent),
    ),
    radioTheme: RadioThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? t.leaf : t.subtle,
      ),
      splashRadius: 0,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      overlayColor: const WidgetStatePropertyAll(Colors.transparent),
    ),
    sliderTheme: SliderThemeData(
      activeTrackColor: t.leaf,
      inactiveTrackColor: t.surface2,
      thumbColor: t.leaf,
      overlayColor: t.leaf.withValues(alpha: 0.12),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: t.leaf,
      linearTrackColor: t.surface2,
      circularTrackColor: t.surface2,
    ),
    tabBarTheme: TabBarThemeData(
      labelColor: t.text,
      unselectedLabelColor: t.muted,
      indicatorColor: t.leaf,
      dividerColor: t.border,
      labelStyle: const TextStyle(fontWeight: FontWeight.w600),
    ),
  );
}
