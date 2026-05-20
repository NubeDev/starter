// Locale fallback resolver (D-NP.6 — `examples/notes/user-pref.md`).
//
// Pure function — same code path runs in `<IntlProvider>` and in
// `locale-fallback.test.ts` so the rule is verified end-to-end.
//
// Algorithm:
//
//   1. Split the requested BCP-47 tag on `-`.
//   2. Try the full tag against the catalog manifest.
//   3. If absent, drop the last segment and retry (`es-MX` → `es`).
//   4. Repeat until either a catalog hits or the segment list is
//      empty.
//   5. If nothing matched, return the `en` floor — provided `en` is
//      itself in the manifest. (`en` is required to be in every
//      starter binary per the prefs SCOPE R5.)
//   6. If even `en` is missing, return `null` so the provider can
//      surface a hard error instead of rendering with no catalog.
//
// `requested === undefined` is treated as "ask for the floor"; the
// resolver returns the same shape as if `requested === "en"`.

import type { I18nManifest, LanguageTag } from "./types.js";

/** Hard-coded fallback per SCOPE R5 — every starter binary ships `en`.
 *  Kept here so the resolver does not import the provider. */
export const I18N_FALLBACK_LANGUAGE: LanguageTag = "en";

export interface LocaleFallbackResult {
  /** The tag that actually has a catalog (the "active" language). */
  picked: LanguageTag;
  /** The fingerprint matching `picked` in the manifest. */
  fingerprint: string;
  /** The chain the resolver walked, in order. The first element is
   *  the originally-requested tag; the last is `picked`. Empty when
   *  `requested` was undefined. Used by the provider to detect
   *  "fell back" and emit `i18n.locale_fallback` exactly once. */
  chain: ReadonlyArray<LanguageTag>;
  /** True when `picked !== requested` (post-empty-normalisation). The
   *  provider's telemetry hook gates on this. */
  fallbackUsed: boolean;
}

/** Resolve a requested BCP-47 tag against a manifest. Returns `null`
 *  when neither the requested tag, any left-truncation of it, nor the
 *  `en` floor is in the manifest — that is a hard misconfiguration
 *  (no `en` catalog shipped) and the provider should surface it.
 *
 *  Lower-cased for matching: BCP-47 says language tags are case-
 *  insensitive. Manifests we ship use lower-case anyway, but consumer
 *  catalogs in the wild may not — defending against this is cheap. */
export function resolveLocale(
  requested: LanguageTag | undefined,
  manifest: I18nManifest,
): LocaleFallbackResult | null {
  // Normalise the manifest keys to lower-case once. The original case
  // is preserved as the returned `picked`, so the URL the provider
  // fetches is the on-disk spelling.
  const byLower = new Map<string, { tag: LanguageTag; fingerprint: string }>();
  for (const [tag, fp] of Object.entries(manifest)) {
    byLower.set(tag.toLowerCase(), { tag, fingerprint: fp });
  }

  const chain: LanguageTag[] = [];

  if (typeof requested === "string" && requested.length > 0) {
    const segments = requested.split("-");
    while (segments.length > 0) {
      const tag = segments.join("-");
      chain.push(tag);
      const hit = byLower.get(tag.toLowerCase());
      if (hit) {
        return {
          picked: hit.tag,
          fingerprint: hit.fingerprint,
          chain: Object.freeze(chain.slice()),
          fallbackUsed: chain.length > 1,
        };
      }
      segments.pop();
    }
  }

  // No tag in the chain matched. Fall to the floor.
  const floor = byLower.get(I18N_FALLBACK_LANGUAGE);
  if (!floor) return null;
  chain.push(I18N_FALLBACK_LANGUAGE);
  return {
    picked: floor.tag,
    fingerprint: floor.fingerprint,
    chain: Object.freeze(chain.slice()),
    fallbackUsed:
      typeof requested === "string" &&
      requested.toLowerCase() !== I18N_FALLBACK_LANGUAGE.toLowerCase(),
  };
}
