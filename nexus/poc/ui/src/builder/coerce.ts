// Turn a raw form string into the JSON value ArkFlow expects for a field kind.

import type { FieldKind } from "../api/catalog";

export function coerceField(kind: FieldKind, raw: string): unknown {
  const value = raw.trim();
  if (value === "") return undefined;

  switch (kind) {
    case "number":
      return Number(value);
    case "bool":
      return value === "true";
    case "list":
      // One entry per line or comma; ArkFlow wants a JSON array of strings.
      return value
        .split(/[\n,]/)
        .map((s) => s.trim())
        .filter(Boolean);
    default:
      // text | duration | code — passed through as a string.
      return value;
  }
}
