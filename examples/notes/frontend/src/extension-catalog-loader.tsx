// `<ExtensionCatalogLoader>` — Stage-5 host-side glue that fetches
// each loaded extension's i18n catalog for the **currently active
// language only** and merges it into the host's `<IntlProvider>` via
// `registerExtensionMessages` (D-NP.8 lazy load).
//
// Lifecycle:
//
// 1. Mount once inside the prefs/i18n shell. The component watches
//    `useIntlContext().language`.
// 2. On every language change, it iterates the module-level
//    `EXTENSION_CATALOGS` map populated by `extension-host.ts` during
//    `registerOne`. For each extension that declares the requested
//    language, it `fetch`es `/extensions/<id>/i18n/<lang>.json` and
//    pipes the JSON through `registerExtensionMessages`.
// 3. Per-(extension, language) in-flight requests are de-duplicated;
//    successful fetches are remembered in a Set so a re-mount or a
//    language flip-back doesn't issue a duplicate request (the HTTP
//    cache covers the byte cost; this avoids the redundant
//    registry mutation + provider re-render).
//
// Failure mode: a single missing catalog (404, network) is logged
// and skipped; the panel renders against react-intl's fallback
// behaviour (the message key string). The provider as a whole stays
// usable.

import { useEffect } from "react";
import { useIntlContext, registerExtensionMessages } from "@nube/starter-ui-core/i18n";
import type { StarterClient } from "@nube/starter-client-ts";

import { _listExtensionCatalogsForTesting } from "./extension-host.js";

/** Marks (extension, language) tuples we've already fetched in this
 * session so we never re-issue the same network request. We don't
 * key on the registry version because the catalog files themselves
 * are immutable for the life of the bundle. */
const FETCHED = new Set<string>();

/** Test helper — wipe the per-session fetched set. */
export function _resetExtensionCatalogFetchesForTesting(): void {
  FETCHED.clear();
}

export interface ExtensionCatalogLoaderProps {
  client: StarterClient;
}

/** Headless effect component. Renders nothing; reacts to language
 * changes to lazy-load extension catalogs. */
export function ExtensionCatalogLoader({
  client,
}: ExtensionCatalogLoaderProps): null {
  const { language } = useIntlContext();

  useEffect(() => {
    let cancelled = false;
    const catalogs = _listExtensionCatalogsForTesting();
    for (const [extensionId, manifest] of catalogs.entries()) {
      if (!manifest.catalogs[language]) continue;
      const key = `${extensionId}|${language}`;
      if (FETCHED.has(key)) continue;
      FETCHED.add(key);
      const url = `${client.baseUrl}/extensions/${encodeURIComponent(
        extensionId,
      )}/i18n/${encodeURIComponent(language)}.json`;
      void (async () => {
        try {
          const res = await client.fetch(url, { headers: client.headers });
          if (!res.ok) {
            // Drop from the "fetched" set so a later language flip
            // back will retry — useful when an extension is enabled
            // between flips.
            FETCHED.delete(key);
            // eslint-disable-next-line no-console
            console.warn(
              `[notes] extension ${extensionId} catalog ${language} → HTTP ${res.status}`,
            );
            return;
          }
          const messages = (await res.json()) as Record<string, string>;
          if (cancelled) return;
          registerExtensionMessages({ extensionId, language, messages });
        } catch (err) {
          FETCHED.delete(key);
          // eslint-disable-next-line no-console
          console.warn(
            `[notes] extension ${extensionId} catalog ${language} failed:`,
            err,
          );
        }
      })();
    }
    return () => {
      cancelled = true;
    };
  }, [client, language]);

  return null;
}
