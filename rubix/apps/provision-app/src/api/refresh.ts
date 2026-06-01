import { useSyncExternalStore } from 'react'

// A shared refresh signal: every mutation bumps it, list views subscribe and
// re-fetch. Mirrors the extension's provision/refresh.ts so a just-added
// device/page/site shows up without a reload (paired with fresh:true reads).
let version = 0
const listeners = new Set<() => void>()

function notify() {
  version += 1
  for (const l of listeners) l()
}

// Fires immediately, then staggered re-reads. A read issued microseconds
// after a write can land on a pooled DB connection that hasn't yet observed
// the commit, briefly missing the just-created row (the read-after-write
// window — ~100ms, occasionally a few seconds). The delayed passes re-read
// once the write is visible, so lists converge without the user reloading.
// Reads are cheap and fresh-dedup'd, so over-firing is harmless.
export function bumpRefresh() {
  notify()
  for (const ms of [300, 800, 1500, 3000, 5000]) setTimeout(notify, ms)
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb)
  return () => listeners.delete(cb)
}

/** Re-renders (and thus re-fetches) the caller whenever any mutation lands. */
export function useRefreshKey(): number {
  return useSyncExternalStore(subscribe, () => version)
}
