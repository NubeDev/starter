// `refresh.ts` — a tiny global "data version" store the provisioning
// tabs share. Mutations (provision, site/page/device CRUD) call
// `bumpRefresh()`; list views subscribe with `useRefreshKey()` and
// re-fetch whenever the version changes.
//
// Why this exists: each tab holds its list in local `useState` and
// fetches on mount. A mutation in one tab (e.g. the wizard provisioning
// a device) left sibling tabs showing stale data until a full page
// reload. A single monotonically-increasing version is the smallest
// fix that makes every list converge after any write — no query
// library, no cross-tab prop threading.

import * as React from "react";
import { invalidateReads } from "../api";

let version = 0;
const listeners = new Set<() => void>();

function notify(): void {
  // Bust the read dedup cache FIRST, so the re-fetches triggered by the
  // listeners below cannot coalesce onto a pre-mutation in-flight read.
  invalidateReads();
  version += 1;
  for (const l of listeners) l();
}

/**
 * Increment the data version and notify every subscribed list.
 *
 * Fires twice — immediately and again after a short delay — because a
 * read issued microseconds after a write can land on a pooled DB
 * connection that hasn't yet observed the commit, briefly missing the
 * just-created row (e.g. a freshly provisioned page). The delayed
 * second pass re-reads once the write is visible, so lists converge
 * without the user having to reload.
 */
export function bumpRefresh(): void {
  // Staggered re-reads cover the window where a pooled DB connection
  // hasn't yet observed the just-committed write. Measured convergence
  // after a provision is up to ~4s, so the tail runs out to 5s. Reads
  // are cheap (small list queries) and dedup-fresh, so over-firing is
  // harmless — it just guarantees the list catches up without a reload.
  notify();
  for (const ms of [300, 800, 1500, 3000, 5000]) setTimeout(notify, ms);
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

/**
 * Returns the current data version. Re-renders the calling component
 * whenever `bumpRefresh()` is called. Use it in an effect dependency
 * list to re-run a fetch after any mutation:
 *
 *   const refresh = useRefreshKey();
 *   React.useEffect(load, [load, refresh]);
 */
export function useRefreshKey(): number {
  return React.useSyncExternalStore(
    subscribe,
    () => version,
    () => version,
  );
}
