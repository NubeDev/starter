export type WidgetType = "line" | "area" | "gauge" | "stat" | "status" | "table";

export type Trend = "up" | "down" | "flat";

export interface WidgetLayout {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface WidgetConfig {
  /** Logical metric/series key — drives the deterministic fake data. */
  metric: string;
  unit?: string;
  /** hsl string for the series colour, e.g. "152 76% 44%". */
  color?: string;
  min?: number;
  max?: number;
  /** For gauge / stat thresholds. */
  warn?: number;
  crit?: number;
  decimals?: number;
}

export interface Widget {
  id: string;
  type: WidgetType;
  title: string;
  subtitle?: string;
  layout: WidgetLayout;
  config: WidgetConfig;
}

export interface Dashboard {
  id: string;
  name: string;
  slug: string;
  description?: string;
  /** lucide icon name */
  icon: string;
  /** accent hsl, e.g. "152 76% 44%" */
  accent: string;
  starred?: boolean;
  widgets: Widget[];
  updatedAt: string;
}
