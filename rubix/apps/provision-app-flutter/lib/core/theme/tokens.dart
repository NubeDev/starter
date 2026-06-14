import 'package:flutter/material.dart';

/// Grid Pulse design tokens — ported verbatim from the React app's
/// `src/index.css` `@theme` block. Obsidian base, electric-teal primary,
/// alert-amber secondary. These are the static palette constants; the live
/// accent (which a device/connection status can override) is resolved per
/// frame by the `look` provider, not here.
abstract final class RubixTokens {
  // ── base ────────────────────────────────────────────────────────────────
  static const obsidian = Color(0xFF07090B);

  static const surface = Color(0xFF0E1113);
  static const surfaceLowest = Color(0xFF090B0D);
  static const surfaceLow = Color(0xFF14181B);
  static const surfaceBase = Color(0xFF181D20);
  static const surfaceHigh = Color(0xFF222A2E);

  // ── ink (on-surface) ──────────────────────────────────────────────────────
  static const ink = Color(0xFFE7F0EF);
  static const inkVariant = Color(0xFFB9C7C6);
  static const inkMuted = Color(0xFF7C8A8A);

  // ── primary = electric teal (the "power on / live" accent) ────────────────
  static const primary = Color(0xFF36E2C4);
  static const primaryContainer = Color(0xFF1F7E6F);
  static const primaryOn = Color(0xFF002019);
  static const primaryDim = Color(0xFF2BC7AD);

  // ── secondary = alert amber (warning / attention) ─────────────────────────
  static const coral = Color(0xFFFFC24B);
  static const coralContainer = Color(0xFF7A3C00);
  static const coralOn = Color(0xFF2A1700);

  // ── status colors — energy apps lean on these for live signalling ─────────
  static const online = Color(0xFF36E2C4);
  static const fault = Color(0xFFFF5A52);
  static const offline = Color(0xFF7C8A8A);

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
  static const fill = Color(0x0FFFFFFF); // rgba(255,255,255,0.06)
  static const border = Color(0x1FFFFFFF); // rgba(255,255,255,0.12)
  static const blur = 24.0;

  static const strongFill = Color(0xB80E1113); // rgba(14,17,19,0.72)
  static const strongBorder = Color(0x1AFFFFFF); // rgba(255,255,255,0.10)
  static const strongBlur = 28.0;
}
