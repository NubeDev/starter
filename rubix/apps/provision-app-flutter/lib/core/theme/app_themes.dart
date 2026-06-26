import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';

/// App theme — the long-term skin. Ported from the React app's `theme/themes.ts`.
/// Re-skinned from the design system's couple themes to an energy/IoT domain.
/// Every widget reads the resolved look via the `look` provider, never these
/// objects directly. The transient "mood" layer is repurposed as device/
/// connection STATUS (see [statuses.dart]) which tints the live accent.
enum ThemeKey { dna }

@immutable
class AppTheme {
  const AppTheme({
    required this.key,
    required this.label,
    required this.blurb,
    required this.icon,
    required this.base,
    required this.accent,
    required this.accent2,
    required this.accentOn,
    required this.accent2On,
    required this.ink,
    required this.inkSoft,
    required this.inkMuted,
    required this.gradient,
    required this.radius,
    required this.glowAlpha,
  });

  final ThemeKey key;
  final String label;
  final String blurb;
  final IconData icon;

  /// page base behind the gradient
  final Color base;

  /// primary accent (status may override the live accent)
  final Color accent;

  /// secondary accent
  final Color accent2;

  /// foreground on top of [accent] (e.g. dark text on a teal button)
  final Color accentOn;

  /// foreground on top of [accent2]
  final Color accent2On;

  /// headline text tint
  final Color ink;

  /// body text tint
  final Color inkSoft;

  /// muted text tint — labels, captions, inactive icons
  final Color inkMuted;

  /// three radial-gradient stops = the ambient base look
  final List<RadialWash> gradient;

  final double radius;
  final double glowAlpha;
}

/// One radial-gradient wash stop — Flutter's analogue of a single CSS
/// `radial-gradient(<size> at <pos>, <color>, transparent <stop>)`.
@immutable
class RadialWash {
  const RadialWash({
    required this.color,
    required this.alignment,
    required this.radius,
    this.stop = 0.7,
  });

  final Color color;
  final Alignment alignment;

  /// gradient radius as a fraction of the shorter side
  final double radius;

  /// where the color fades to transparent
  final double stop;
}

/// The single DNA theme — dark obsidian base, soft electric-teal accent, and
/// three ambient teal radial washes matching the glow ellipses on every Figma
/// frame (top-right, mid-right, bottom-left). This is now the *only* skin; the
/// former grid/solar/offpeak/industrial themes were retired in the DNA reskin.
const themes = <ThemeKey, AppTheme>{
  ThemeKey.dna: AppTheme(
    key: ThemeKey.dna,
    label: 'DNA',
    blurb: 'Teal · provision',
    icon: LucideIcons.activity,
    base: Color(0xFF0B0F10), // dark obsidian DNA base
    accent: Color(0xFF61A3AE), // DNA brand teal (muted)
    accent2: Color(0xFFFBBD41), // yellow callout
    accentOn: Color(0xFF04181C),
    accent2On: Color(0xFF241200),
    ink: Color(0xFFE7F0EF),
    inkSoft: Color(0xFFB9C7C6),
    inkMuted: Color(0xFF7E8888),
    gradient: [
      RadialWash(
        color: Color(0x2E61A3AE), // teal glow, top-right · ~0.18
        alignment: Alignment(0.9, -1.0),
        radius: 0.95,
      ),
      RadialWash(
        color: Color(0x2461A3AE), // teal glow, mid-right · ~0.14
        alignment: Alignment(1.0, 0.3),
        radius: 0.8,
      ),
      RadialWash(
        color: Color(0x2A4497A2), // deep-teal glow, bottom-left · ~0.16
        alignment: Alignment(-1.1, 1.1),
        radius: 1.0,
      ),
    ],
    radius: 20,
    glowAlpha: 0.4,
  ),
};

const themeOrder = <ThemeKey>[ThemeKey.dna];
const defaultTheme = ThemeKey.dna;

ThemeKey? themeKeyFromName(String? name) =>
    ThemeKey.values.where((k) => k.name == name).firstOrNull;
