// `lib/detail.ts` — module-level dedupe for `GET /api/v1/extensions/<id>`.
//
// Several components in this bundle (the main panel header + the
// sidebar version badge) need the same extension-detail payload. A
// naive `fetch()` per component yields N identical requests on every
// page load. This helper coalesces concurrent callers onto a single
// in-flight promise and caches the resolved value for the lifetime of
// the module — i.e. for the lifetime of the loaded federation remote.
//
// Call `invalidateExtensionDetail()` from any explicit "refresh"
// button to force the next call to re-fetch.

import type { ExtensionDetail } from "../types";

import { EXTENSION_ID } from "../types";

let cached: ExtensionDetail | null = null;
let inFlight: Promise<ExtensionDetail> | null = null;

export function fetchExtensionDetail(): Promise<ExtensionDetail> {
  if (cached) return Promise.resolve(cached);
  if (inFlight) return inFlight;
  // The host's bootstrap loop already fetched `/extensions/<id>` and
  // stashed the payload here. Federation modules run in their own
  // module scope, so we can't read the host's locals — but we share
  // the same `window`. Prefer this over a duplicate network round-trip.
  const g = globalThis as unknown as {
    __starterExtensionDetailCache__?: Record<string, ExtensionDetail>;
  };
  const fromHost = g.__starterExtensionDetailCache__?.[EXTENSION_ID];
  if (fromHost) {
    cached = fromHost;
    return Promise.resolve(fromHost);
  }
  inFlight = fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
    credentials: "same-origin",
    headers: { accept: "application/json" },
  })
    .then(async (res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const detail = (await res.json()) as ExtensionDetail;
      cached = detail;
      return detail;
    })
    .finally(() => {
      inFlight = null;
    });
  return inFlight;
}

export function invalidateExtensionDetail(): void {
  cached = null;
  inFlight = null;
  const g = globalThis as unknown as {
    __starterExtensionDetailCache__?: Record<string, ExtensionDetail>;
  };
  if (g.__starterExtensionDetailCache__) {
    delete g.__starterExtensionDetailCache__[EXTENSION_ID];
  }
}
