import type { SeriesField } from "@/data/types";

// ECharts paints to canvas and cannot resolve CSS custom properties
// (`var(--chart-1)`) — it needs concrete colour strings. We read the
// theme's chart ramp off the document root once and cache it, so series
// colours still come from the theme tokens (F6) rather than being
// hardcoded, but reach ECharts as resolved values. Falls back to a fixed
// emerald-led ramp when there is no DOM (tests/SSR).
const FALLBACK = [
  "#22c55e",
  "#38bdf8",
  "#a78bfa",
  "#fbbf24",
  "#fb7185",
] as const;

let cachedRamp: string[] | null = null;

function readRamp(): string[] {
  if (cachedRamp) return cachedRamp;
  if (typeof document === "undefined") return [...FALLBACK];
  const style = getComputedStyle(document.documentElement);
  const ramp = [1, 2, 3, 4, 5].map((n, i) => {
    const v = style.getPropertyValue(`--chart-${n}`).trim();
    return v || FALLBACK[i];
  });
  cachedRamp = ramp;
  return ramp;
}

/** Apply an alpha to a resolved colour for ECharts gradient stops. Hex
 *  (`#rrggbb`) gets an alpha suffix; `hsl(...)` / `oklch(...)` are wrapped
 *  with a `/ alpha`. Already-functional colours pass through. */
export function withAlpha(color: string, alpha: number): string {
  const a = Math.max(0, Math.min(1, alpha));
  if (color.startsWith("#") && color.length === 7) {
    const hex = Math.round(a * 255)
      .toString(16)
      .padStart(2, "0");
    return `${color}${hex}`;
  }
  const fn = color.match(/^(hsl|oklch|rgb)\((.+)\)$/i);
  if (fn) return `${fn[1]}(${fn[2]} / ${a})`;
  return color;
}

/** Resolve a series' colour to a concrete value ECharts can paint: an
 *  explicit hsl config wins; otherwise the theme ramp slot for its
 *  index, wrapping past five series. */
export function seriesColor(field: SeriesField, index: number): string {
  if (field.color) return `hsl(${field.color})`;
  const ramp = readRamp();
  return ramp[index % ramp.length];
}

/** The resolved threshold-state colours for gauges, read from the same
 *  theme tokens. */
export function stateColor(state: "ok" | "warn" | "crit"): string {
  const ramp = readRamp();
  if (state === "crit") return ramp[4];
  if (state === "warn") return ramp[3];
  return ramp[0];
}

// Resolved chrome colours (axis lines, gridlines, gauge track, labels)
// that ECharts paints onto canvas — the same role tokens the rest of the
// UI uses, read once from the document root. Fixed fallbacks match the
// OLED palette for tests/SSR.
const CHROME_FALLBACK: Record<string, string> = {
  "--border": "#1e2a3a",
  "--muted": "#1a2536",
  "--muted-foreground": "#8a98ad",
  "--foreground": "#f4f8fb",
};

let cachedChrome: Record<string, string> | null = null;

export function chromeColor(token: keyof typeof CHROME_FALLBACK): string {
  if (!cachedChrome) {
    cachedChrome =
      typeof document === "undefined"
        ? { ...CHROME_FALLBACK }
        : Object.fromEntries(
            Object.keys(CHROME_FALLBACK).map((k) => {
              const v = getComputedStyle(document.documentElement)
                .getPropertyValue(k)
                .trim();
              return [k, v || CHROME_FALLBACK[k]];
            }),
          );
  }
  return cachedChrome[token];
}

/** Drop the cached chart ramp + chrome colours so the next read re-pulls
 *  them from the document root. The theme store calls this on every
 *  dark/light switch, so canvas-painted ECharts series re-tint to the new
 *  palette instead of keeping the colours captured at first render. */
export function invalidateThemeCache(): void {
  cachedRamp = null;
  cachedChrome = null;
}
