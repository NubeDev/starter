// DOM-free core of the preferences module.
//
// Stage 1 of the rubix-mobile workspace refactor extracted everything
// that does not touch the DOM out of `provider.tsx` so React-Native
// consumers can import the context, hook, constants and HTTP helpers
// without dragging `document.documentElement` (lang/dir writes) or
// JSX-with-`<div>` (the aria-live announcer) into the Hermes bundle.
//
// The web `PreferencesProvider` in `./provider.tsx` re-exports every
// symbol from this file so existing callers (and `./index.ts`) keep
// the same import surface. RN consumers import directly from
// `./provider-core.js` via a future native barrel — they never reach
// the `.tsx` sibling.

import { createContext, useContext } from "react";
import type { StarterClient } from "@nube/starter-client-ts";

import type { PreferencesPatch, ResolvedPreferences } from "./types.js";

/** Name of the same-browser multi-tab fan-out channel. Frozen on
 *  merge per `examples/notes/user-pref.md` D-NP.9; production-team
 *  dashboards and dev tooling key off this exact string. */
export const PREFERENCES_BROADCAST_CHANNEL = "starter-prefs";

/** Message shape posted on the BroadcastChannel. The receiver
 *  invalidates the query (server is the source of truth) and
 *  optimistically applies the patch via `setQueryData` so the flip
 *  appears in the next animation frame; the subsequent refetch is
 *  the safety net. */
export interface PreferencesBroadcastMessage {
  kind: "starter-prefs:patch";
  workspaceId: string;
  patch: PreferencesPatch;
}

/** Workspace sentinel matching the Rust resolver's default scope. */
export const DEFAULT_WORKSPACE = "@starter/default";

export interface PreferencesContextValue {
  /** The resolved preferences. `null` while the initial fetch is in
   * flight. Consumers can branch on `null` to render a loading state
   * or simply skip rendering until preferences arrive. */
  preferences: ResolvedPreferences | null;
  /** True while the initial fetch is in flight. */
  isLoading: boolean;
  /** Fetch error, if the initial probe failed. */
  error: unknown;
  /** PATCH a subset of fields; resolves once the server has applied
   * the change and the query has refetched. A `null` field value
   * means "revert to inherit" per the Rust route layer. */
  setPreferences: (patch: PreferencesPatch) => Promise<void>;
}

/**
 * The React Context object backing `<PreferencesProvider>`. Exported
 * so the host's Module-Federation runtime can register it as the
 * `@nube/starter-ui-core/preferences` singleton — extensions then
 * `useContext(handle.singletons["@nube/starter-ui-core/preferences"])`
 * against the host's instance instead of bundling their own copy.
 */
export const PreferencesContext = createContext<PreferencesContextValue | undefined>(undefined);

/** Read the preferences context. Throws if called outside a
 * `<PreferencesProvider>`. */
export function usePreferences(): PreferencesContextValue {
  const ctx = useContext(PreferencesContext);
  if (!ctx) throw new Error("usePreferences must be called inside <PreferencesProvider>");
  return ctx;
}

/** Visually hidden, screen-reader-only style. Inline because the
 *  provider must not depend on a global stylesheet. */
export const SR_ONLY_STYLE = {
  position: "absolute",
  width: "1px",
  height: "1px",
  margin: "-1px",
  padding: 0,
  overflow: "hidden",
  clip: "rect(0,0,0,0)",
  whiteSpace: "nowrap",
  border: 0,
} as const;

/** Build the polite-announcement string for a language flip. Tries
 *  `Intl.DisplayNames` in the new language (so a flip to `es` says
 *  `"Idioma cambiado a Español"`); falls back to the BCP-47 tag if
 *  the runtime lacks `DisplayNames` or the lookup throws. */
export function buildLanguageAnnouncement(language: string): string {
  try {
    const dn = new Intl.DisplayNames([language], { type: "language" });
    const localized = dn.of(language);
    // Match the SCOPE-named phrasing: `"Idioma cambiado a Español"`
    // in the *new* language. We rely on `Intl.DisplayNames` for the
    // language name; the verb is statically picked per language from
    // a tiny table so consumers don't have to ship a catalog for
    // this single string. Unknown languages fall back to English.
    const phrase = LANG_CHANGED_PHRASE[language.split("-")[0]?.toLowerCase() ?? ""];
    if (phrase && localized) return `${phrase} ${localized}`;
    return localized
      ? `Language changed to ${localized}`
      : `Language changed to ${language}`;
  } catch {
    return `Language changed to ${language}`;
  }
}

/** Tiny localised lead-in for the aria-live announcement. Adding a
 *  catalog round-trip just for this one string would defeat the
 *  point — the announcer fires *during* a catalog flip. */
export const LANG_CHANGED_PHRASE: Record<string, string> = {
  en: "Language changed to",
  es: "Idioma cambiado a",
  de: "Sprache geändert zu",
  fr: "Langue changée en",
  pt: "Idioma alterado para",
  it: "Lingua cambiata in",
  nl: "Taal gewijzigd in",
  ja: "言語が変更されました:",
  zh: "语言已更改为",
};

/** Pure helper: BCP-47 → RTL flag. Extracted out of the DOM-bound
 *  `useEffect` in `<PreferencesProvider>` so RN consumers (which read
 *  the resolved language directly off `usePreferences()`) can apply
 *  the same RTL rule when they wire `I18nManager.forceRTL`. */
export function isRtlLanguage(language: string): boolean {
  const primary = language.split("-")[0]?.toLowerCase();
  return primary === "ar" || primary === "he" || primary === "fa" || primary === "ur";
}

// ---------------------------------------------------------------------
// HTTP — minimal direct fetches via StarterClient. We do not depend on
// a generated endpoint module because /v1/me/preferences ships with
// starter-prefs, which sits outside the codegen pipeline today. The
// shape is hand-mirrored in `types.ts`.
// ---------------------------------------------------------------------

export async function fetchMyPreferences(
  client: StarterClient,
  workspaceId: string,
): Promise<ResolvedPreferences> {
  const url = `${client.baseUrl}/v1/me/preferences?org=${encodeURIComponent(workspaceId)}`;
  const res = await client.fetch(url, {
    credentials: "include",
    headers: client.headers,
  });
  if (!res.ok) {
    throw new Error(`GET /v1/me/preferences failed: ${res.status}`);
  }
  return (await res.json()) as ResolvedPreferences;
}

export async function patchMyPreferences(
  client: StarterClient,
  workspaceId: string,
  patch: PreferencesPatch,
): Promise<void> {
  const url = `${client.baseUrl}/v1/me/preferences?org=${encodeURIComponent(workspaceId)}`;
  const res = await client.fetch(url, {
    method: "PATCH",
    credentials: "include",
    headers: { ...client.headers, "content-type": "application/json" },
    body: JSON.stringify(patch),
  });
  if (!res.ok) {
    throw new Error(`PATCH /v1/me/preferences failed: ${res.status}`);
  }
}
