// Tunables & static palettes for the dashboard.
//
// Kept here (and re-exported through `index.tsx`) so component
// modules don't reach back into the orchestrator file to read a
// constant.

export type MeterKind = "elec" | "water";

export const RANGES = [
  { label: "24h", hours: 24,    bucket: "15 minutes" },
  { label: "7d",  hours: 168,   bucket: "1 hour"     },
  { label: "30d", hours: 720,   bucket: "6 hours"    },
  { label: "90d", hours: 2160,  bucket: "1 day"      },
  // 6 months ≈ 26 weeks. Daily bucket keeps payload < ~180 rows/host.
  { label: "6m",  hours: 4368,  bucket: "1 day"      },
  { label: "1y",  hours: 8760,  bucket: "1 day"      },
] as const;

export const KINDS: ReadonlyArray<{
  kind: MeterKind;
  label: string;
  secondaryTag: string;
  hint: string;
  /** Display unit hint for KPIs when the data layer doesn't infer one. */
  unitHint: string;
}> = [
  { kind: "elec",  label: "Electrical", secondaryTag: "power",   hint: "Instantaneous power (kW)",       unitHint: "kW"     },
  { kind: "water", label: "Water",      secondaryTag: "reading", hint: "Meter reading (litres / kL)",     unitHint: "litres" },
];

// Palette for series lines — distinct, accessible on dark.
export const PALETTE = [
  "#2dd4bf", "#60a5fa", "#f472b6", "#facc15", "#a78bfa",
  "#34d399", "#fb7185", "#22d3ee", "#fdba74", "#c084fc",
  "#4ade80", "#f87171", "#38bdf8", "#fcd34d", "#e879f9",
];

export const DAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

// Threshold above which the per-site tile grid collapses to a
// scalable portfolio table.
export const TILES_MAX = 12;

// Cap on per-site overlay lines in the main chart (besides "Total").
export const SERIES_TOP_N = 5;

export const MAX_SAVED_VIEWS = 12;
