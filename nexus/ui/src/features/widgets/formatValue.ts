import type { FieldDisplay, ValueMapping } from "@/data/types";
import { unitDef } from "@/features/widgets/units";

// Turns a raw value into the display string a stat/gauge/table cell
// shows: applies value mappings first (a matched mapping can replace the
// text outright), then fixed decimals, then the unit symbol. Pure — same
// inputs always produce the same string — so it is unit-testable and
// reusable across every renderer without pulling React or the theme.

/** The result of formatting: the display text plus an optional colour a
 *  matched value-mapping requested (hsl string), which the caller may
 *  paint the cell/number with. */
export interface FormattedValue {
  text: string;
  color?: string;
}

/** Format `value` for display under `display` (unit/decimals/mappings).
 *  `null`/`undefined`/non-finite values render `display.noValue` (default
 *  em dash) so panels never show "NaN". */
export function formatValue(
  value: number | string | null | undefined,
  display: FieldDisplay | undefined = {},
): FormattedValue {
  const mapped = matchMapping(value, display.mappings);
  if (mapped && (mapped.text != null || mapped.color != null)) {
    // A mapping that supplies text wins outright; one that supplies only a
    // colour still falls through to numeric formatting for the text.
    if (mapped.text != null) return { text: mapped.text, color: mapped.color };
  }

  if (value == null || (typeof value === "number" && !Number.isFinite(value))) {
    return { text: display.noValue ?? "—", color: mapped?.color };
  }

  const num = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(num)) {
    // Non-numeric string with no mapping: show it verbatim.
    return { text: String(value), color: mapped?.color };
  }

  const text = formatNumber(num, display);
  return { text, color: mapped?.color };
}

// Apply decimals + unit symbol to a finite number.
function formatNumber(num: number, display: FieldDisplay): string {
  const u = unitDef(display.unit);
  // `percentunit` stores 0–1 but shows 0–100.
  const scaled = display.unit === "percentunit" ? num * 100 : num;
  const fixed =
    display.decimals == null
      ? trimAuto(scaled)
      : scaled.toFixed(display.decimals);
  if (!u || !u.symbol) return fixed;
  if (u.prefix) return `${u.symbol}${fixed}`;
  return u.space ? `${fixed} ${u.symbol}` : `${fixed}${u.symbol}`;
}

// Auto precision: integers print whole, fractions keep up to 2 places
// without trailing zeros — a sane default when no `decimals` is set.
function trimAuto(n: number): string {
  if (Number.isInteger(n)) return String(n);
  return String(Math.round(n * 100) / 100);
}

// First matching value mapping (by declaration order), or undefined.
function matchMapping(
  value: number | string | null | undefined,
  mappings: ReadonlyArray<ValueMapping> | undefined,
): ValueMapping | undefined {
  if (!mappings || mappings.length === 0) return undefined;
  const str = value == null ? "" : String(value);
  const num = typeof value === "number" ? value : Number(value);
  return mappings.find((m) => {
    if (m.type === "value") return m.match != null && m.match === str;
    if (m.type === "regex") {
      if (m.match == null) return false;
      try {
        return new RegExp(m.match).test(str);
      } catch {
        return false;
      }
    }
    // range
    if (!Number.isFinite(num)) return false;
    if (m.from != null && num < m.from) return false;
    if (m.to != null && num > m.to) return false;
    return m.from != null || m.to != null;
  });
}
