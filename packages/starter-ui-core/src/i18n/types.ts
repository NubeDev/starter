// Wire shapes for the i18n catalog endpoints served by
// `starter-i18n` (`GET /v1/i18n/manifest` and
// `GET /v1/i18n/catalogs/{lang}-{fingerprint}.json`). Hand-mirrored
// for the same reason as `preferences/types.ts`: starter-i18n is not
// in the codegen pipeline yet.

/** A BCP-47 language tag carried as a free string on the wire. */
export type LanguageTag = string;

/** A reverse-DNS-style dotted message key — e.g.
 * `starter.settings.preferences.timezone.label`. */
export type MessageKey = string;

/** `GET /v1/i18n/manifest` → `{ "en": "ab12…", "es": "cd34…" }`.
 * Each value is the 16-char fingerprint locked at stage 13. */
export type I18nManifest = Readonly<Record<LanguageTag, string>>;

/** `GET /v1/i18n/catalogs/{lang}-{fp}.json` → a flat map of
 * `MessageKey` → ICU MessageFormat string. */
export type Catalog = Readonly<Record<MessageKey, string>>;
