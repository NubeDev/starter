// Serialise variable selections into URL query params and back, for
// shareable deep links (item 8). Param names mirror Grafana's `var-<name>`
// convention so links read familiarly; a multi-select repeats the param
// (`?var-region=us&var-region=eu`). Pure string<->state mapping — the React
// glue lives in `useVariableUrlSync`.

/** Selections keyed by variable name (without `$`), each an ordered value
 *  list. A single-select carries one entry; multi carries several. */
export type VariableSelections = Record<string, string[]>;

const PREFIX = "var-";

/** Read every `var-<name>` out of a query string into a selections map.
 *  Repeated params accumulate (multi-select). Returns only the keys
 *  present, so the caller keeps a variable's stored default when its param
 *  is absent rather than clobbering it to empty. */
export function parseVariableParams(params: URLSearchParams): VariableSelections {
  const out: VariableSelections = {};
  for (const [key, value] of params.entries()) {
    if (!key.startsWith(PREFIX)) continue;
    const name = key.slice(PREFIX.length);
    if (!name) continue;
    (out[name] ??= []).push(value);
  }
  return out;
}

/** Apply selections onto a params object (mutating a copy): drops every
 *  existing `var-*` first, then writes one param per value. Returns the
 *  updated params so the caller can compare/replace. A variable with no
 *  selection contributes no param, keeping the URL clean. */
export function writeVariableParams(
  base: URLSearchParams,
  selections: VariableSelections,
): URLSearchParams {
  const next = new URLSearchParams();
  // Preserve non-variable params (from/to/refresh, etc.) in their order.
  for (const [key, value] of base.entries()) {
    if (!key.startsWith(PREFIX)) next.append(key, value);
  }
  // Append variable params in name order for a deterministic URL.
  for (const name of Object.keys(selections).sort()) {
    for (const value of selections[name]) next.append(`${PREFIX}${name}`, value);
  }
  return next;
}
