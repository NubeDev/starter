/**
 * Row-template binding — substitutes `{{$row.*}}` tokens in a
 * component subtree with values extracted from a `UiTableRow`. Used
 * by the table component when rendering per-row children defined
 * once as a template at the table node.
 *
 * Rules:
 * - Only string values are scanned for tokens; other values pass
 *   through unchanged.
 * - A `"{{...}}"` that matches the whole string value preserves the
 *   JS type (number, boolean, null) by writing the raw JSON.
 * - An embedded `{{...}}` inside a larger string coerces to
 *   `String(v)`; missing paths produce `""`.
 * - The input subtree is never mutated (JSON round-trip clone).
 */
import type { UiComponent, UiTableRow } from "./types.js";

export function bindRow(template: UiComponent, row: UiTableRow): UiComponent {
  const json = JSON.stringify(template);
  const bound = json.replace(/"{{(\$row\.[^}]+)}}"/g, (_, expr: string) => {
    const v = resolveExpr(expr, row);
    if (v === undefined || v === null) return "null";
    if (typeof v === "string") return JSON.stringify(v);
    if (typeof v === "number" || typeof v === "boolean") return String(v);
    return JSON.stringify(v);
  });
  const embedded = bound.replace(/{{(\$row\.[^}]+)}}/g, (_, expr: string) => {
    const v = resolveExpr(expr, row);
    if (v === undefined || v === null) return "";
    return String(v);
  });
  return JSON.parse(embedded) as UiComponent;
}

function resolveExpr(expr: string, row: UiTableRow): unknown {
  if (!expr.startsWith("$row.")) return undefined;
  const path = expr.slice("$row.".length);
  const parts = path.split(".");

  if (parts[0] === "slots") {
    if (parts.length < 2) return undefined;
    // Longest match first — slots may be stored under dotted keys
    // (`"settings.title"`) or as a single key with a nested value.
    for (let len = parts.length - 1; len >= 1; len--) {
      const slotKey = parts.slice(1, len + 1).join(".");
      if (slotKey in row.slots) {
        const value = row.slots[slotKey];
        const remaining = parts.slice(len + 1);
        if (remaining.length === 0) return value;
        return extractDotPath(value, remaining);
      }
    }
    return undefined;
  }

  if (parts.length === 1) {
    return (row as unknown as Record<string, unknown>)[parts[0]!];
  }
  return extractDotPath(row as unknown, parts);
}

function extractDotPath(v: unknown, parts: string[]): unknown {
  let cursor: unknown = v;
  for (const part of parts) {
    if (cursor == null || typeof cursor !== "object") return undefined;
    cursor = (cursor as Record<string, unknown>)[part];
  }
  return cursor;
}
