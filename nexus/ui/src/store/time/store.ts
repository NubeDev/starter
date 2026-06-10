import { create } from "zustand";

import type { TimeRange } from "@/store/time/resolve";

// Dashboard-global time-range + auto-refresh state. Ephemeral client state
// (like the UI store) — server data stays in TanStack Query. `zustand`'s
// `create` is imported from the workspace's single federation singleton so
// host and remotes share one store runtime.
//
// `tick` is the frozen-`now` nonce: each auto-refresh fire (or manual
// refresh) bumps both `tick` and `now`, and the query layer keys on `tick`
// so relative ranges re-resolve and the cache busts exactly once per
// interval — not on every render. Freezing one `now` per tick gives every
// panel in a refresh the same instant (no fan-out clock skew).

/** Refresh interval in seconds; `0` means off (manual only). */
export type RefreshSecs = number;

interface TimeState {
  /** The range as the user expressed it (relative tokens kept verbatim). */
  range: TimeRange;
  /** Auto-refresh interval in seconds (`0` = off). */
  refresh: RefreshSecs;
  /** Monotonic refresh counter; bumped on each refresh fire. */
  tick: number;
  /** The frozen reference instant for the current `tick`. */
  now: Date;
  setRange: (range: TimeRange) => void;
  setRefresh: (refresh: RefreshSecs) => void;
  /** Advance to a fresh `now` + `tick`: the auto-refresh loop and the manual
   *  refresh button both call this. */
  bump: () => void;
}

/** Default window: the last 6 hours, refresh off. */
export const DEFAULT_RANGE: TimeRange = { from: "now-6h", to: "now" };

export const useTimeStore = create<TimeState>((set) => ({
  range: DEFAULT_RANGE,
  refresh: 0,
  tick: 0,
  now: new Date(),
  // Changing the range freezes a new instant immediately so panels re-run
  // against a coherent snapshot without waiting for the next tick.
  setRange: (range) => set((s) => ({ range, tick: s.tick + 1, now: new Date() })),
  setRefresh: (refresh) => set({ refresh }),
  bump: () => set((s) => ({ tick: s.tick + 1, now: new Date() })),
}));
