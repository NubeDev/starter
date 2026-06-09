import { unitDef } from "@/features/widgets/units";

// Splits a resolved unit into the prefix/suffix slots `MetricCard` renders
// around its animated number. A registry id resolves to its symbol and
// placement; an unknown id is treated as a raw trailing symbol (so the
// legacy `SeriesField.unit`, which held bare strings like "rps", still
// shows). Pure.
export interface UnitSymbol {
  prefix?: string;
  suffix?: string;
}

export function unitSymbol(unit: string | undefined): UnitSymbol {
  if (!unit) return {};
  const def = unitDef(unit);
  if (!def) {
    // A bare legacy symbol string: render it as a trailing suffix verbatim.
    return { suffix: unit };
  }
  if (!def.symbol) return {};
  return def.prefix ? { prefix: def.symbol } : { suffix: def.symbol };
}
