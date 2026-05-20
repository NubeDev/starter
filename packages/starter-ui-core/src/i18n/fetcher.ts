// Thin fetch helpers for the i18n endpoints. Kept separate from the
// React component so the provider stays testable with a mock fetch
// injected via the shared `StarterClient`.

import type { StarterClient } from "@nube/starter-client-ts";

import type { Catalog, I18nManifest, LanguageTag } from "./types.js";

/** `GET /v1/i18n/manifest` — returns `{ lang: fingerprint }`. */
export async function fetchManifest(client: StarterClient): Promise<I18nManifest> {
  const res = await client.fetch(`${client.baseUrl}/v1/i18n/manifest`, {
    headers: client.headers,
  });
  if (!res.ok) {
    throw new Error(`GET /v1/i18n/manifest failed: ${res.status}`);
  }
  return (await res.json()) as I18nManifest;
}

/** `GET /v1/i18n/catalogs/{lang}-{fingerprint}.json` — the
 * fingerprinted URL is immutable; safe to cache forever. */
export async function fetchCatalog(
  client: StarterClient,
  language: LanguageTag,
  fingerprint: string,
): Promise<Catalog> {
  const url = `${client.baseUrl}/v1/i18n/catalogs/${encodeURIComponent(language)}-${encodeURIComponent(fingerprint)}.json`;
  const res = await client.fetch(url, { headers: client.headers });
  if (!res.ok) {
    throw new Error(`GET ${url} failed: ${res.status}`);
  }
  return (await res.json()) as Catalog;
}

/** Module-level cache of fingerprinted catalog payloads. The
 * fingerprint is content-addressed, so a cache hit is always sound;
 * we never evict. */
const CATALOG_CACHE = new Map<string, Promise<Catalog>>();

/** Wrap `fetchCatalog` with a permanent in-memory cache keyed by the
 * fingerprinted URL. The promise is cached so concurrent requests
 * collapse into one network call. */
export function loadCatalogCached(
  client: StarterClient,
  language: LanguageTag,
  fingerprint: string,
): Promise<Catalog> {
  const key = `${client.baseUrl}|${language}-${fingerprint}`;
  const hit = CATALOG_CACHE.get(key);
  if (hit) return hit;
  const promise = fetchCatalog(client, language, fingerprint).catch((err) => {
    // Don't poison the cache on failure — let the next call retry.
    CATALOG_CACHE.delete(key);
    throw err;
  });
  CATALOG_CACHE.set(key, promise);
  return promise;
}

/** Test helper — wipe the module-level catalog cache. */
export function _resetCatalogCacheForTesting(): void {
  CATALOG_CACHE.clear();
}
