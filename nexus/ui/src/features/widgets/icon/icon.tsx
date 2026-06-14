import {
  AreaChart,
  BarChart3,
  CircleDot,
  Gauge,
  Grid3x3,
  Hash,
  PieChart,
  ScatterChart,
  Table,
  TrendingUp,
  type LucideIcon,
} from "lucide-react";

// Resolves the catalog's icon *names* (kept as strings so `catalog.ts`
// stays data-only and React-free) to lucide components. Only the icons
// the catalog references are imported, so this map is the one place a new
// widget type's icon is bound to a component. Falls back to a neutral
// chart glyph for an unknown name rather than crashing the palette.
const ICONS: Record<string, LucideIcon> = {
  TrendingUp,
  AreaChart,
  BarChart3,
  ScatterChart,
  Grid3x3,
  PieChart,
  Gauge,
  Hash,
  CircleDot,
  Table,
};

export function widgetIcon(name: string): LucideIcon {
  return ICONS[name] ?? TrendingUp;
}
