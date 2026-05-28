// `detail.ts` — module-level dedupe for `GET /api/v1/extensions/<id>`.
//
// Both the Main panel and the Sidebar mount on the same page load and
// each wants the same extension-detail payload (version, contributes
// counts). Without coalescing we issue N identical requests. This
// helper caches the resolved value at module scope (i.e. for the
// lifetime of the loaded federation remote) and de-dupes concurrent
// callers onto a single in-flight promise.

import type { ExtensionDetail } from "./types";
import { EXTENSION_ID } from "./types";

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
