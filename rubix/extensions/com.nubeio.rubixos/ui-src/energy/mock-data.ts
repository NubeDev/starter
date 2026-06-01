// Mock data for the Energy dashboard.
//
// IMPORTANT: v1 is a presentation page. Wiring this to live warehouse
// templates is a follow-up — the extension's templates are tuned for
// BMS power / water reading data, not energy generation (PV, wind,
// hydro). Mock numbers are tuned to LOOK realistic on screen.

export type SourceKind = "solar" | "wind" | "water" | "hydro" | "storage";

export interface SeriesPoint {
  /** Bucket label — short hour code for the 24h timeline. */
  t: string;
  solar: number;
  wind: number;
  water: number;
  hydro: number;
}

export interface SiteRow {
  name: string;
  region: string;
  kind: SourceKind;
  /** kWh generated today. */
  total: number;
  /** Fraction 0..1 of the portfolio leader (used for bar gauge). */
  share: number;
}

export interface WeatherSlot {
  day: string;
  icon: "sun" | "cloud" | "rain" | "wind";
  high: number;
  low: number;
  wind: number;
}

export interface LiveSourceStatus {
  kind: SourceKind;
  label: string;
  online: number;
  offline: number;
  status: "ok" | "warn" | "fault";
  detail: string;
}

/** Total kWh today (headline KPI). */
export const TOTAL_KWH_TODAY = 184_320;

/** Delta vs yesterday (+12.4%). */
export const TOTAL_KWH_DELTA_PCT = 12.4;

/** CO2 avoided in kg today. */
export const CO2_AVOIDED_KG = 73_280;
export const CO2_DELTA_PCT = 8.7;

/** Mix shares (percent) for the donut. Sums to 100. */
export const MIX_SHARE = [
  { kind: "solar" as const,  label: "Solar",  value: 46, color: "#fbbf24" },
  { kind: "wind"  as const,  label: "Wind",   value: 28, color: "#22d3ee" },
  { kind: "water" as const,  label: "Water",  value: 14, color: "#38bdf8" },
  { kind: "hydro" as const,  label: "Hydro",  value: 12, color: "#6366f1" },
];

/** 24-hour generation timeline. Solar bell-curves at midday, wind is
 *  noisier evening-heavy, water/hydro near baseload. */
export const TIMELINE_24H: ReadonlyArray<SeriesPoint> = buildTimeline();

function buildTimeline(): SeriesPoint[] {
  const rows: SeriesPoint[] = [];
  for (let h = 0; h < 24; h++) {
    const solarBell = Math.max(0, Math.sin(((h - 6) / 12) * Math.PI));
    const solar = Math.round(2400 * solarBell + (Math.random() * 120 - 60));
    const windNoise = 0.6 + 0.4 * Math.sin(h / 3) + (Math.random() * 0.25 - 0.12);
    const wind = Math.round(1100 * Math.max(0.3, windNoise));
    const water = Math.round(420 + Math.sin(h / 4) * 60 + (Math.random() * 30 - 15));
    const hydro = Math.round(380 + Math.cos(h / 5) * 40 + (Math.random() * 24 - 12));
    rows.push({
      t: h.toString().padStart(2, "0") + ":00",
      solar: Math.max(0, solar),
      wind,
      water,
      hydro,
    });
  }
  return rows;
}

/** Top 5 sites by generation today. */
export const TOP_SITES: ReadonlyArray<SiteRow> = [
  { name: "Tarcoola Solar Farm",  region: "SA · AU",  kind: "solar", total: 42_180, share: 1.00 },
  { name: "Macarthur Wind",       region: "VIC · AU", kind: "wind",  total: 31_640, share: 0.75 },
  { name: "Snowy 2.0 Tumut",      region: "NSW · AU", kind: "hydro", total: 22_910, share: 0.54 },
  { name: "Murray Run-of-River",  region: "VIC · AU", kind: "water", total: 18_440, share: 0.44 },
  { name: "Coopers Gap Wind",     region: "QLD · AU", kind: "wind",  total: 14_220, share: 0.34 },
];

/** Live status per source. */
export const LIVE_STATUS: ReadonlyArray<LiveSourceStatus> = [
  { kind: "solar",   label: "Inverters online",       online: 312, offline: 4, status: "ok",   detail: "98.7% uptime · 4 in maintenance" },
  { kind: "wind",    label: "Turbines spinning",      online:  74, offline: 2, status: "ok",   detail: "Avg rotor: 14.2 rpm" },
  { kind: "water",   label: "Flow stations",          online:  28, offline: 0, status: "ok",   detail: "All sites within target flow" },
  { kind: "hydro",   label: "Hydro penstocks",        online:  12, offline: 1, status: "warn", detail: "Tumut-3 throttled (debris)" },
  { kind: "storage", label: "Battery stacks armed",   online:   8, offline: 0, status: "ok",   detail: "State of charge 71%" },
];

/** 5-day weather strip — drives the planning narrative. */
export const WEATHER_5D: ReadonlyArray<WeatherSlot> = [
  { day: "Mon", icon: "sun",   high: 32, low: 19, wind: 12 },
  { day: "Tue", icon: "sun",   high: 34, low: 21, wind:  8 },
  { day: "Wed", icon: "cloud", high: 28, low: 18, wind: 22 },
  { day: "Thu", icon: "wind",  high: 26, low: 17, wind: 38 },
  { day: "Fri", icon: "rain",  high: 23, low: 16, wind: 28 },
];

/** Storage tile: battery state-of-charge percent. */
export const BATTERY_SOC_PCT = 71;
export const BATTERY_RATE_KW = 4_200; // discharging

/** Accent CSS colours per source — referenced from JS for chart fills. */
export const ACCENT: Record<SourceKind, { from: string; to: string; text: string; ring: string }> = {
  solar:   { from: "#fbbf24", to: "#f59e0b", text: "text-amber-300",  ring: "ring-amber-400/40" },
  wind:    { from: "#22d3ee", to: "#0891b2", text: "text-cyan-300",   ring: "ring-cyan-400/40"  },
  water:   { from: "#38bdf8", to: "#0284c7", text: "text-sky-300",    ring: "ring-sky-400/40"   },
  hydro:   { from: "#818cf8", to: "#4338ca", text: "text-indigo-300", ring: "ring-indigo-400/40"},
  storage: { from: "#a3e635", to: "#65a30d", text: "text-lime-300",   ring: "ring-lime-400/40"  },
};

/** Headline gen-today spark for the hero tile. */
export const HERO_SPARK = TIMELINE_24H.map((p) => p.solar + p.wind + p.water + p.hydro);
