// Tiny URL query-string sync used by the dashboard + report pages,
// so views like "?range=6m&kind=water" are shareable and bookmarkable.
//
// Why a hand-rolled hook (not React Router):
//   • The extension is mounted inside the host frontend's
//     `extensions/$extId/$` catch-all route, so we don't own the
//     router. We can only touch `window.location` / `history`.
//   • Reading once on mount + writing on change with `replaceState`
//     is all we need; no nav events to subscribe to from the host.

import * as React from "react";

import { KINDS, RANGES, type MeterKind } from "./presets";

/** Parsed query-string values understood by the dashboard pages. */
export interface UrlState {
  kindIdx: number | null;
  rangeIdx: number | null;
}

function rangeIdxFromLabel(label: string | null): number | null {
  if (!label) return null;
  const i = RANGES.findIndex((r) => r.label.toLowerCase() === label.toLowerCase());
  return i >= 0 ? i : null;
}

function kindIdxFromName(name: string | null): number | null {
  if (!name) return null;
  const n = name.toLowerCase();
  // Accept both the canonical kind ("elec"/"water") and the human
  // labels ("electrical"/"energy"/"water") so shared links read
  // naturally.
  const aliases: Record<string, MeterKind> = {
    elec: "elec", electrical: "elec", electricity: "elec", energy: "elec", power: "elec",
    water: "water", h2o: "water",
  };
  const kind = aliases[n];
  if (!kind) return null;
  const i = KINDS.findIndex((k) => k.kind === kind);
  return i >= 0 ? i : null;
}

/**
 * One-shot read of the current URL (`?range=…&kind=…`).
 *
 * Returns `null` for any param the user didn't supply or that doesn't
 * map to a known value — the caller keeps its existing default in
 * that case.
 */
export function readUrlState(): UrlState {
  if (typeof window === "undefined") return { kindIdx: null, rangeIdx: null };
  const q = new URLSearchParams(window.location.search);
  return {
    kindIdx: kindIdxFromName(q.get("kind")),
    rangeIdx: rangeIdxFromLabel(q.get("range")),
  };
}

/**
 * Mirror `kindIdx` / `rangeIdx` into the URL query string using
 * `history.replaceState` (no navigation, no router involvement).
 *
 * Skips the write on the very first render so we don't strip
 * unrelated query params the host may have set; from there on, every
 * change is pushed back to the URL so the bar is always shareable.
 *
 * Pass `kindIdx: null` for pages that don't expose a kind dimension
 * (e.g. the report which always shows both channels) — only `range`
 * is written in that case.
 */
export function useUrlSync(kindIdx: number | null, rangeIdx: number): void {
  const first = React.useRef(true);
  React.useEffect(() => {
    if (first.current) { first.current = false; return; }
    if (typeof window === "undefined") return;
    const url = new URL(window.location.href);
    if (kindIdx !== null) url.searchParams.set("kind", KINDS[kindIdx]!.kind);
    url.searchParams.set("range", RANGES[rangeIdx]!.label);
    window.history.replaceState(window.history.state, "", url.toString());
  }, [kindIdx, rangeIdx]);
}
