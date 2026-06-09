// Serialise the time-range + refresh into URL query params and back, for
// shareable deep links (C3). Pure string<->state mapping — the React glue
// lives in `useTimeUrlSync`. Param names mirror Grafana's (`from`/`to`/
// `refresh`) so links read familiarly; `refresh` is in seconds (`0`/absent
// = off).

import type { RefreshSecs } from "@/store/time/store";
import type { TimeRange } from "@/store/time/resolve";

/** The slice of state reflected in the URL. */
export interface TimeUrlState {
  range: TimeRange;
  refresh: RefreshSecs;
}

/** Read `from`/`to`/`refresh` out of a query string. Returns `undefined`
 *  fields when absent so the caller keeps its current/default value rather
 *  than clobbering it. */
export function parseTimeParams(params: URLSearchParams): {
  range?: TimeRange;
  refresh?: RefreshSecs;
} {
  const from = params.get("from");
  const to = params.get("to");
  const refreshRaw = params.get("refresh");

  const out: { range?: TimeRange; refresh?: RefreshSecs } = {};
  if (from && to) out.range = { from, to };
  if (refreshRaw !== null) {
    const n = Number(refreshRaw);
    if (Number.isFinite(n) && n >= 0) out.refresh = n;
  }
  return out;
}

/** Apply the time state onto a params object (mutating a copy), dropping
 *  `refresh` when off so an off dashboard has a clean URL. Returns the
 *  updated params so the caller can compare/replace. */
export function writeTimeParams(
  base: URLSearchParams,
  state: TimeUrlState,
): URLSearchParams {
  const next = new URLSearchParams(base);
  next.set("from", state.range.from);
  next.set("to", state.range.to);
  if (state.refresh > 0) next.set("refresh", String(state.refresh));
  else next.delete("refresh");
  return next;
}
