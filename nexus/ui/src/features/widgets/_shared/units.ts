// The unit registry the panel editor's Field tab picks from, and the
// formatter the renderers use to turn a raw number into a display string
// (value + symbol). Deliberately a small, data-only client-side table:
// it carries *display* symbols and a suffix/prefix rule, not conversion.
//
// Conversion + per-user quantity preferences are WS-11's concern (the
// Field tab's unit will later become a `quantity` whose displayed unit
// comes from the viewer's resolved prefs). Until then this is a flat
// display-only registry, kept pure so it can be unit-tested and reused by
// the table/stat/gauge renderers without pulling React.

/** A selectable display unit: an id stored on the field config, a label
 *  for the picker, the symbol appended (or prepended) to the value, and
 *  whether the symbol leads (currency) or trails (most units). */
export interface UnitDef {
  id: string;
  label: string;
  symbol: string;
  /** `true` → symbol before the number ("$12"); default trails ("12 °C"). */
  prefix?: boolean;
  /** A space between number and symbol. Currencies/percent omit it. */
  space?: boolean;
}

/** A named group of units for the picker's grouped dropdown. */
export interface UnitGroup {
  label: string;
  units: ReadonlyArray<UnitDef>;
}

// Grouped so the picker renders section headers (SI, data rate, temp, …)
// the way Grafana's unit menu does. Ids are stable wire values stored in
// `FieldDisplay.unit`; changing an id is a breaking change to saved panels.
export const UNIT_GROUPS: ReadonlyArray<UnitGroup> = [
  {
    label: "Misc",
    units: [{ id: "none", label: "None", symbol: "" }],
  },
  {
    label: "Percentage",
    units: [
      { id: "percent", label: "Percent (0–100)", symbol: "%" },
      { id: "percentunit", label: "Percent (0.0–1.0)", symbol: "%" },
    ],
  },
  {
    label: "Temperature",
    units: [
      { id: "celsius", label: "Celsius", symbol: "°C", space: true },
      { id: "fahrenheit", label: "Fahrenheit", symbol: "°F", space: true },
      { id: "kelvin", label: "Kelvin", symbol: "K", space: true },
    ],
  },
  {
    label: "Energy & power",
    units: [
      { id: "watt", label: "Watt", symbol: "W", space: true },
      { id: "kilowatt", label: "Kilowatt", symbol: "kW", space: true },
      { id: "watthour", label: "Watt-hour", symbol: "Wh", space: true },
      { id: "kilowatthour", label: "Kilowatt-hour", symbol: "kWh", space: true },
      { id: "volt", label: "Volt", symbol: "V", space: true },
      { id: "ampere", label: "Ampere", symbol: "A", space: true },
    ],
  },
  {
    label: "Data",
    units: [
      { id: "bytes", label: "Bytes", symbol: "B", space: true },
      { id: "kbytes", label: "Kibibytes", symbol: "KiB", space: true },
      { id: "mbytes", label: "Mebibytes", symbol: "MiB", space: true },
      { id: "gbytes", label: "Gibibytes", symbol: "GiB", space: true },
    ],
  },
  {
    label: "Data rate",
    units: [
      { id: "bps", label: "Bits/sec", symbol: "bps", space: true },
      { id: "Bps", label: "Bytes/sec", symbol: "B/s", space: true },
    ],
  },
  {
    label: "Time",
    units: [
      { id: "ms", label: "Milliseconds", symbol: "ms", space: true },
      { id: "s", label: "Seconds", symbol: "s", space: true },
      { id: "m", label: "Minutes", symbol: "min", space: true },
      { id: "h", label: "Hours", symbol: "h", space: true },
    ],
  },
  {
    label: "Currency",
    units: [
      { id: "usd", label: "US Dollar", symbol: "$", prefix: true },
      { id: "eur", label: "Euro", symbol: "€", prefix: true },
      { id: "gbp", label: "Pound", symbol: "£", prefix: true },
    ],
  },
];

// Flat id → def lookup, derived once from the groups so the groups stay
// the single source of truth.
const UNIT_BY_ID: ReadonlyMap<string, UnitDef> = new Map(
  UNIT_GROUPS.flatMap((g) => g.units).map((u) => [u.id, u]),
);

/** Resolve a unit id to its definition, or undefined for an unknown id
 *  (so a stale saved id degrades to unitless rather than throwing). */
export function unitDef(id: string | undefined): UnitDef | undefined {
  return id ? UNIT_BY_ID.get(id) : undefined;
}
