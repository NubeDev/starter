import 'package:flutter/material.dart';

/// Rubix app theme built from seed color per DECISIONS.md.
const _seedColor = Color(0xFF1F2A2E);

/// Light theme.
final ThemeData rubixLightTheme = ThemeData(
  useMaterial3: true,
  colorScheme: ColorScheme.fromSeed(seedColor: _seedColor),
);

/// Dark theme.
final ThemeData rubixDarkTheme = ThemeData(
  useMaterial3: true,
  colorScheme: ColorScheme.fromSeed(
    seedColor: _seedColor,
    brightness: Brightness.dark,
  ),
);
