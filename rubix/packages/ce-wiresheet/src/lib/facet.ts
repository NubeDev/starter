// Per-component `__facet` presentation metadata: parse / serialize / cache.
//
// `__facet` is just an input string property on each component (systemRole
// ROLE_FACETS) whose value describes how to present the component's OTHER
// properties — labels, units, number formatting, and value→label aliases.
// See FACET_DESIGN.md for the full design. Control-char delimited so it's a
// cheap split-parse (no JSON); both the engine and the UI write it.

// Delimiters (never appear in user text → no escaping).
const RS = "\x1e"; // between property records
const US = "\x1f"; // between fields within a record
const GS = "\x1d"; // between alias / option items
const FS = "\x1c"; // between an alias's code and its label

// The property name that carries the facet string on every component.
export const FACET_PROP = "__facets";

export interface Alias {
  code: number; // the property's native value (int; bool → 0/1)
  label: string;
}

export interface PropFacet {
  label?: string;
  unit?: string;
  decimals?: number;
  min?: number;
  max?: number;
  hidden?: boolean;
  order?: number;
  aliases?: Alias[]; // `o` — value→label map (also the pick list)
  action?: string; // `a` — dynamic-options action (Phase 3; not used yet)
}

export type ComponentFacet = Map<number /* propUid */, PropFacet>;

export function parseFacet(raw: string): ComponentFacet {
  const out: ComponentFacet = new Map();
  if (!raw) return out;
  for (const rec of raw.split(RS)) {
    if (!rec) continue;
    const fields = rec.split(US);
    const uid = Number(fields[0]);
    if (!Number.isFinite(uid)) continue;
    const f: PropFacet = {};
    for (let i = 1; i < fields.length; i++) {
      const fld = fields[i];
      if (!fld) continue;
      const v = fld.slice(1);
      switch (fld[0]) {
        case "l": f.label = v; break;
        case "u": f.unit = v; break;
        case "d": f.decimals = Number(v); break;
        case "n": f.min = Number(v); break;
        case "x": f.max = Number(v); break;
        case "h": f.hidden = v !== "0"; break;
        case "r": f.order = Number(v); break;
        case "a": f.action = v; break;
        case "o":
          f.aliases = v.split(GS).map((o) => {
            const j = o.indexOf(FS);
            return j < 0
              ? { code: Number(o), label: o }
              : { code: Number(o.slice(0, j)), label: o.slice(j + 1) };
          });
          break;
      }
    }
    out.set(uid, f);
  }
  return out;
}

export function serializeFacet(facet: ComponentFacet): string {
  const recs: string[] = [];
  for (const [uid, f] of facet) {
    const fields: string[] = [String(uid)];
    if (f.label) fields.push("l" + f.label);
    if (f.unit) fields.push("u" + f.unit);
    if (f.decimals != null) fields.push("d" + f.decimals);
    if (f.min != null) fields.push("n" + f.min);
    if (f.max != null) fields.push("x" + f.max);
    if (f.hidden) fields.push("h1");
    if (f.order != null) fields.push("r" + f.order);
    if (f.action) fields.push("a" + f.action);
    if (f.aliases && f.aliases.length) {
      fields.push("o" + f.aliases.map((a) => a.code + FS + a.label).join(GS));
    }
    if (fields.length > 1) recs.push(fields.join(US)); // skip empty records
  }
  return recs.join(RS);
}

// Per-component parse cache — re-parse only when a component's raw facet string
// actually changes. Bounded by the number of components.
const cache = new Map<number, { raw: string; parsed: ComponentFacet }>();

export function facetFor(componentUid: number, raw: string | undefined): ComponentFacet {
  const key = raw ?? "";
  const hit = cache.get(componentUid);
  if (hit && hit.raw === key) return hit.parsed;
  const parsed = parseFacet(key);
  cache.set(componentUid, { raw: key, parsed });
  return parsed;
}

// Read the raw facet string off a component's REST properties (or undefined).
export function rawFacet(
  properties: Record<string, { value: unknown }> | undefined,
): string | undefined {
  const v = properties?.[FACET_PROP]?.value;
  return typeof v === "string" ? v : undefined;
}

// Resolve a property's native value to its alias label, if the facet aliases it.
export function aliasLabel(aliases: Alias[] | undefined, value: unknown): string | undefined {
  if (!aliases || aliases.length === 0) return undefined;
  const code =
    value === true ? 1 : value === false ? 0 : typeof value === "number" ? value : Number(value);
  return aliases.find((a) => a.code === code)?.label;
}
