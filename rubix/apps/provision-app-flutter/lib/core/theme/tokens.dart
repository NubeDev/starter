import 'package:flutter/material.dart';

/// Design-system primitives. Two kinds of value live here:
///
///  1. **Invariant tokens** — the radius scale, spacing, glass fills, and the
///     fixed status colors (`fault`/`online`/`offline`). These never change
///     with the theme; read them directly anywhere.
///  2. **The static baseline palette** (`primary`/`ink`/`surface`/… — the DNA
///     theme) — used ONLY where a runtime `Look` is unreachable: the Material
///     [ColorScheme] in `buildRubixTheme`, the `const` [RubixText] styles, and
///     a couple of provider-side fallbacks.
///
/// EVERYTHING in the widget tree should read the *per-theme* palette from the
/// `look` provider / `context.look` instead — that's the layer that re-skins
/// when the theme or live status changes. Don't reach for these palette
/// constants in new feature code; reach for `context.look`.
abstract final class RubixTokens {
  // ── base ── DNA kit: base #0B0F10, card #121819 ───────────────────────────
  static const obsidian = Color(0xFF0B0F10);

  static const surface = Color(0xFF121819); // "card"
  static const surfaceLowest = Color(0xFF090C0D);
  static const surfaceLow = Color(0xFF151B1C);
  static const surfaceBase = Color(0xFF1A2123);
  static const surfaceHigh = Color(0xFF232B2D);

  // ── ink (on-surface) ──────────────────────────────────────────────────────
  static const ink = Color(0xFFE7F0EF);
  static const inkVariant = Color(0xFFB9C7C6);
  static const inkMuted = Color(0xFF7E8888); // kit caption grey

  // ── primary = DNA teal (the heading-accent + "live" accent) ───────────────
  static const primary = Color(0xFF61A3AE); // DNA kit "teal"
  static const primaryContainer = Color(0xFF2D6971);
  static const primaryOn = Color(0xFF04181C);
  static const primaryDim = Color(0xFF4497A2);

  // ── secondary = DNA yellow (callout / underline / small badges) ───────────
  static const coral = Color(0xFFFBB93E); // DNA kit "yellow"
  static const coralContainer = Color(0xFF76360F);
  static const coralOn = Color(0xFF241200);

  // ── status colors — semantic dots (connected/warning/offline/idle) ────────
  static const online = Color(0xFF5DCAA5); // kit "green" · connected
  static const fault = Color(0xFFE24B4A); // kit "red" · offline/alarm
  static const warning = Color(0xFFC9A24A); // kit "amber" · warning
  static const offline = Color(0xFF7E8888); // kit grey · idle

  // ── radii (rem → logical px, 1rem = 16) ──────────────────────────────────
  static const radiusSm = 4.0;
  static const radius = 8.0;
  static const radiusMd = 12.0;
  static const radiusLg = 16.0;
  static const radiusXl = 24.0;
  static const radius2xl = 28.0;

  // ── spacing ───────────────────────────────────────────────────────────────
  static const margin = 20.0;
  static const gutter = 12.0;

  static const blurGlass = 24.0;
}

/// Glass surface fills + borders, factored out so the glass widgets and the
/// bottom sheet/toast share one source of truth (the React `@utility glass`
/// and `glass-strong`).
abstract final class Glass {
  // DNA kit: "Glass surface · 6% fill · 12px blur".
  static const fill = Color(0x0FFFFFFF); // rgba(255,255,255,0.06)
  static const border = Color(0x1FFFFFFF); // rgba(255,255,255,0.12)
  static const blur = 12.0;

  static const strongFill = Color(0xB8121819); // card @ 0.72
  static const strongBorder = Color(0x1AFFFFFF); // rgba(255,255,255,0.10)
  static const strongBlur = 16.0;
}
