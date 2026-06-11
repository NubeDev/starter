import type {
  FieldDisplay,
  FieldOverride,
  SeriesField,
  ThresholdStep,
  WidgetConfig,
} from "@/data/types";

// Resolves a panel's field config into the *effective* display for one
// series: the defaults with any matching override laid on top, plus a
// back-compat bridge from the legacy flat `min/max/decimals/thresholds`.
// Pure (no React, no fetch) so the option-builders and the table/stat
// renderers can call it and stay testable.

/** The display config that actually applies to a given series, after
 *  merging defaults, overrides, and the legacy flat fields. Extends
 *  {@link FieldDisplay} with the override-only display extras. */
export interface ResolvedField extends FieldDisplay {
  /** Override-supplied display label (else the series' own label/value). */
  displayName?: string;
  /** Override-supplied series colour (hsl string), wins over the field's
   *  own `color`. */
  color?: string;
  hidden?: boolean;
}

/** Compute the effective display for `series` under `config`. Layers, in
 *  increasing precedence: legacy flat fields → `fieldConfig.defaults` →
 *  the first matching override. The series' own `label`/`color` seed the
 *  display name/colour when no override sets them. */
export function resolveField(
  series: SeriesField,
  config: WidgetConfig,
): ResolvedField {
  const legacy = legacyDefaults(config);
  const defaults = config.fieldConfig?.defaults ?? {};
  const override = matchOverride(series, config.fieldConfig?.overrides);

  const base: ResolvedField = {
    ...legacy,
    ...stripUndefined(defaults),
    unit: defaults.unit ?? series.unit ?? legacy.unit,
    displayName: series.label,
    color: series.color,
  };

  if (!override) return base;
  return {
    ...base,
    ...stripUndefined(override.display),
    displayName: override.display.displayName ?? base.displayName,
    color: override.display.color ?? base.color,
  };
}

/** First override whose matcher selects this series, or undefined. A
 *  `byName` matcher tests the series' `value` column and its `label`;
 *  `byRegex` tests the same against the pattern. */
export function matchOverride(
  series: SeriesField,
  overrides: ReadonlyArray<FieldOverride> | undefined,
): FieldOverride | undefined {
  if (!overrides || overrides.length === 0) return undefined;
  const candidates = [series.value, series.label].filter(
    (s): s is string => typeof s === "string" && s.length > 0,
  );
  return overrides.find((o) => {
    if (o.matcher.type === "byName") {
      return candidates.includes(o.matcher.value);
    }
    try {
      const re = new RegExp(o.matcher.value);
      return candidates.some((c) => re.test(c));
    } catch {
      return false;
    }
  });
}

/** The threshold ramp a value should be classified against: the
 *  `fieldConfig` steps if present, else the legacy warn/crit promoted to
 *  a three-step ramp so existing gauges keep colouring. Returns an empty
 *  array when neither is configured. */
export function resolveThresholdSteps(config: WidgetConfig): ThresholdStep[] {
  const steps = config.fieldConfig?.defaults?.thresholds;
  if (steps && steps.length > 0) return [...steps];
  return [];
}

// Map the legacy flat WidgetConfig fields onto a FieldDisplay so callers
// only ever read one shape. Threshold promotion stays the renderers'
// concern (they still understand the legacy `Thresholds`), so this only
// bridges min/max/decimals.
function legacyDefaults(config: WidgetConfig): FieldDisplay {
  const d: FieldDisplay = {};
  if (config.min != null) d.min = config.min;
  if (config.max != null) d.max = config.max;
  if (config.decimals != null) d.decimals = config.decimals;
  return d;
}

// Drop keys whose value is undefined so a spread doesn't clobber a lower
// layer's defined value with `undefined`.
function stripUndefined<T extends object>(obj: T): Partial<T> {
  const out: Partial<T> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (v !== undefined) (out as Record<string, unknown>)[k] = v;
  }
  return out;
}
