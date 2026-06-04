import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';

/// App theme — the long-term skin. Ported from the React app's `theme/themes.ts`.
/// Re-skinned from the design system's couple themes to an energy/IoT domain.
/// Every widget reads the resolved look via the `look` provider, never these
/// objects directly. The transient "mood" layer is repurposed as device/
/// connection STATUS (see [statuses.dart]) which tints the live accent.
enum ThemeKey { grid, solar, offpeak, industrial }

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

const themes = <ThemeKey, AppTheme>{
  ThemeKey.grid: AppTheme(
    key: ThemeKey.grid,
    label: 'Grid',
    blurb: 'Electric teal · live',
    icon: LucideIcons.activity,
    base: Color(0xFF07090B),
    accent: Color(0xFF36E2C4),
    accent2: Color(0xFFFFC24B),
    accentOn: Color(0xFF002019),
    accent2On: Color(0xFF2A1700),
    ink: Color(0xFFE7F0EF),
    inkSoft: Color(0xFFB9C7C6),
    inkMuted: Color(0xFF7C8A8A),
    gradient: [
      RadialWash(
        color: Color(0x3336E2C4), // rgba(54,226,196,0.20)
        alignment: Alignment(0.6, -1.2),
        radius: 0.9,
      ),
      RadialWash(
        color: Color(0x1FFFC24B), // rgba(255,194,75,0.12)
        alignment: Alignment(-1, -0.84),
        radius: 0.8,
      ),
      RadialWash(
        color: Color(0x2E1F7E6F), // rgba(31,126,111,0.18)
        alignment: Alignment(0, 1.2),
        radius: 1,
      ),
    ],
    radius: 24,
    glowAlpha: 0.45,
  ),
  ThemeKey.solar: AppTheme(
    key: ThemeKey.solar,
    label: 'Solar',
    blurb: 'Amber · daytime yield',
    icon: LucideIcons.sun,
    base: Color(0xFF0C0A06),
    accent: Color(0xFFFFC24B),
    accent2: Color(0xFFFF8F5E),
    accentOn: Color(0xFF2A1700),
    accent2On: Color(0xFF2A1100),
    ink: Color(0xFFFFF3DA),
    inkSoft: Color(0xFFE7D6B4),
    inkMuted: Color(0xFF9A8C6E),
    gradient: [
      RadialWash(
        color: Color(0x47FFC24B), // 0.28
        alignment: Alignment(0.56, -1.16),
        radius: 0.95,
      ),
      RadialWash(
        color: Color(0x2EFF8F5E), // 0.18
        alignment: Alignment(-0.88, -0.8),
        radius: 0.8,
      ),
      RadialWash(
        color: Color(0x29FFD27A), // 0.16
        alignment: Alignment(0, 1.24),
        radius: 1.1,
      ),
    ],
    radius: 24,
    glowAlpha: 0.42,
  ),
  ThemeKey.offpeak: AppTheme(
    key: ThemeKey.offpeak,
    label: 'Off-peak',
    blurb: 'Cool · low demand',
    icon: LucideIcons.moon,
    base: Color(0xFF070A10),
    accent: Color(0xFF8FB6FF),
    accent2: Color(0xFF36E2C4),
    accentOn: Color(0xFF06183A),
    accent2On: Color(0xFF002019),
    ink: Color(0xFFE3ECFF),
    inkSoft: Color(0xFFBCC8E6),
    inkMuted: Color(0xFF7E89A8),
    gradient: [
      RadialWash(
        color: Color(0x3D5A82D2), // rgba(90,130,210,0.24)
        alignment: Alignment(0, -1.2),
        radius: 1,
      ),
      RadialWash(
        color: Color(0x2936E2C4), // 0.16
        alignment: Alignment(-0.72, -0.52),
        radius: 0.9,
      ),
      RadialWash(
        color: Color(0x8C0F162D), // rgba(15,22,45,0.55)
        alignment: Alignment(0, 1.32),
        radius: 1.3,
      ),
    ],
    radius: 20,
    glowAlpha: 0.5,
  ),
  ThemeKey.industrial: AppTheme(
    key: ThemeKey.industrial,
    label: 'Industrial',
    blurb: 'Steel · plant floor',
    icon: LucideIcons.factory,
    base: Color(0xFF0A0B0C),
    accent: Color(0xFF9FB0B8),
    accent2: Color(0xFFFFC24B),
    accentOn: Color(0xFF11171A),
    accent2On: Color(0xFF2A1700),
    ink: Color(0xFFE9EEF0),
    inkSoft: Color(0xFFBCC6CA),
    inkMuted: Color(0xFF7E888C),
    gradient: [
      RadialWash(
        color: Color(0x299FB0B8), // 0.16
        alignment: Alignment(0.56, -1.16),
        radius: 0.9,
      ),
      RadialWash(
        color: Color(0x1AFFC24B), // 0.10
        alignment: Alignment(-0.88, -0.8),
        radius: 0.8,
      ),
      RadialWash(
        color: Color(0x8C283034), // rgba(40,48,52,0.55)
        alignment: Alignment(0, 1.24),
        radius: 1.1,
      ),
    ],
    radius: 16,
    glowAlpha: 0.4,
  ),
};

const themeOrder = <ThemeKey>[
  ThemeKey.grid,
  ThemeKey.solar,
  ThemeKey.offpeak,
  ThemeKey.industrial,
];
const defaultTheme = ThemeKey.grid;

ThemeKey? themeKeyFromName(String? name) =>
    ThemeKey.values.where((k) => k.name == name).firstOrNull;
