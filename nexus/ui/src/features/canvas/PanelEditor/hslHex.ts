// Conversions between the model's hsl triple (e.g. "152 76% 44%", the
// project's `SeriesField.color` convention) and the `#rrggbb` hex an
// `<input type="color">` speaks. Used by the threshold and override
// colour swatches so the control round-trips without changing how colours
// are stored. Pure.

const BRAND_GREEN = "#22c55e";

/** hsl triple → `#rrggbb`. Falls back to the brand green for an empty or
 *  unparseable input so the swatch always shows a valid colour. */
export function hslToHex(hsl: string | undefined): string {
  if (!hsl) return BRAND_GREEN;
  const m = hsl.match(/(-?\d+(?:\.\d+)?)\s+(-?\d+(?:\.\d+)?)%\s+(-?\d+(?:\.\d+)?)%/);
  if (!m) return BRAND_GREEN;
  const h = Number(m[1]);
  const s = Number(m[2]) / 100;
  const l = Number(m[3]) / 100;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const mm = l - c / 2;
  const [r, g, b] = rgbFromHue(h, c, x);
  const to = (n: number) =>
    Math.round((n + mm) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${to(r)}${to(g)}${to(b)}`;
}

/** `#rrggbb` → hsl triple matching the storage convention. */
export function hexToHsl(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    if (max === r) h = (g - b) / d + (g < b ? 6 : 0);
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h *= 60;
  }
  return `${Math.round(h)} ${Math.round(s * 100)}% ${Math.round(l * 100)}%`;
}

function rgbFromHue(h: number, c: number, x: number): [number, number, number] {
  if (h < 60) return [c, x, 0];
  if (h < 120) return [x, c, 0];
  if (h < 180) return [0, c, x];
  if (h < 240) return [0, x, c];
  if (h < 300) return [x, 0, c];
  return [c, 0, x];
}
