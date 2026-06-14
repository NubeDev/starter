import {
  Activity,
  BatteryCharging,
  Boxes,
  Building2,
  Cpu,
  Droplet,
  Factory,
  Flame,
  Gauge,
  LayoutDashboard,
  Leaf,
  Lightbulb,
  Server,
  Snowflake,
  Thermometer,
  Wind,
  Zap,
  type LucideIcon,
} from "lucide-react";

// Dashboard appearance presets — the curated icon set and accent palette
// shared by the create and edit forms. Icons are stored as lucide *names*
// (the wire/DB carries a string), resolved to components here; accents are
// HSL triple strings applied as `hsl(<triple>)`, matching how the theme
// tokens are consumed elsewhere.

// The picker's icon set: identity glyphs that suit energy/water/HVAC pages
// (the product domain), not chart-type icons. Order is the grid order.
export const DASHBOARD_ICONS = [
  "Activity",
  "Gauge",
  "Zap",
  "BatteryCharging",
  "Thermometer",
  "Snowflake",
  "Flame",
  "Droplet",
  "Wind",
  "Lightbulb",
  "Leaf",
  "Factory",
  "Building2",
  "Server",
  "Cpu",
  "Boxes",
] as const;

// The accent swatch palette (HSL triples) — emerald, sky, violet, amber,
// rose. Stored verbatim and applied as `hsl(<triple>)`.
export const DASHBOARD_ACCENTS = [
  "152 76% 44%",
  "199 90% 56%",
  "263 80% 66%",
  "38 95% 56%",
  "346 84% 60%",
] as const;

export const DEFAULT_ICON = DASHBOARD_ICONS[0];
export const DEFAULT_ACCENT = DASHBOARD_ACCENTS[0];

const ICONS: Record<string, LucideIcon> = {
  Activity,
  Gauge,
  Zap,
  BatteryCharging,
  Thermometer,
  Snowflake,
  Flame,
  Droplet,
  Wind,
  Lightbulb,
  Leaf,
  Factory,
  Building2,
  Server,
  Cpu,
  Boxes,
  // A sensible fallback glyph for any name not in the curated set.
  LayoutDashboard,
};

/** Resolve a stored icon name to its lucide component, falling back to a
 *  neutral dashboard glyph for an unknown name rather than crashing. */
export function dashboardIcon(name: string): LucideIcon {
  return ICONS[name] ?? LayoutDashboard;
}
