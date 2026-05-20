// `<PreferencesProvider>` + `usePreferences()` — react-query backed
// access to the caller's `ResolvedPreferences`.
//
// State management choice (Phase 4 stage-1 lock): **react-query +
// React context**, no zustand. The data is server-owned, has a clear
// cache key (`["starter", "preferences", workspaceId]`), and patches
// invalidate → react-query refetches; a separate zustand store would
// duplicate cache state and force us to keep the two in sync. The
// `setPreferences` callback exposed via context PATCHes the server
// and invalidates the query, never mutating local state directly.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { StarterClient } from "@nube/starter-client-ts";

import { starterQueryKey } from "../query/index.js";
import { emitPreferencesTelemetry } from "./telemetry.js";
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
interface PreferencesBroadcastMessage {
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

export interface PreferencesProviderProps {
  /** The shared `StarterClient`. Re-used for the underlying HTTP. */
  client: StarterClient;
  /** Workspace to scope the lookup against. Falls back to the
   * `@starter/default` sentinel so single-tenant deployments work
   * with zero configuration. */
  workspaceId?: string;
  /** Rendered while the initial prefs probe is in flight. Default:
   * an empty fragment. Stage-1 loading contract — formatters never
   * run against unresolved prefs because consumers below this
   * provider only mount once `preferences !== null`. Pass the
   * host's top-bar skeleton here so the FOUC stays on-brand. */
  fallback?: ReactNode;
  children: ReactNode;
}

/** Wire the resolved preferences into a React subtree. Must be
 * nested inside a `<QueryClientProvider>`. */
export function PreferencesProvider({
  client,
  workspaceId = DEFAULT_WORKSPACE,
  fallback = null,
  children,
}: PreferencesProviderProps) {
  const queryClient = useQueryClient();
  const queryKey = useMemo(() => starterQueryKey("preferences", workspaceId), [workspaceId]);

  // BroadcastChannel — D-NP.9 multi-tab propagation. One channel per
  // tab; same-origin browsers fan out automatically. Mounted lazily
  // so SSR / older browsers (no global) simply skip cross-tab sync.
  // `useRef` keeps the channel stable across renders.
  const channelRef = useRef<BroadcastChannel | null>(null);
  useEffect(() => {
    if (typeof BroadcastChannel === "undefined") return;
    let ch: BroadcastChannel;
    try {
      ch = new BroadcastChannel(PREFERENCES_BROADCAST_CHANNEL);
    } catch (err) {
      emitPreferencesTelemetry({
        kind: "prefs.broadcast_dropped",
        severity: "warn",
        patch: {},
        reason: err instanceof Error ? err.message : String(err),
      });
      return;
    }
    channelRef.current = ch;
    ch.onmessage = (ev: MessageEvent<PreferencesBroadcastMessage>) => {
      const data = ev.data;
      if (!data || data.kind !== "starter-prefs:patch") return;
      if (data.workspaceId !== workspaceId) return;
      // Optimistic apply against the cache so the receiving tab
      // re-renders in the next animation frame; the invalidation
      // queues a refetch so the server stays the source of truth.
      const prev = queryClient.getQueryData<ResolvedPreferences>(queryKey);
      if (prev) {
        // Spread the patch but strip explicit nulls (the wire shape's
        // "revert to inherit" sentinel) — local cache holds the
        // resolved view, not the inheritance graph.
        const next = { ...prev } as unknown as Record<string, unknown>;
        for (const [k, v] of Object.entries(data.patch)) {
          if (v != null) next[k] = v;
        }
        queryClient.setQueryData(queryKey, next as unknown as ResolvedPreferences);
      }
      void queryClient.invalidateQueries({ queryKey });
    };
    return () => {
      try {
        ch.close();
      } catch {
        /* nothing observable */
      }
      channelRef.current = null;
    };
  }, [queryClient, queryKey, workspaceId]);

  const query = useQuery<ResolvedPreferences>({
    queryKey,
    queryFn: () => fetchMyPreferences(client, workspaceId),
  });

  const mutation = useMutation({
    mutationFn: (patch: PreferencesPatch) => patchMyPreferences(client, workspaceId, patch),
    onSuccess: (_data, patch) => {
      void queryClient.invalidateQueries({ queryKey });
      // Fan out to other tabs of the same browser. Failures here do
      // not roll the mutation back — the local PATCH already
      // succeeded — but they do fire `prefs.broadcast_dropped` so
      // platform dashboards can spot a hardened-browser regression.
      const ch = channelRef.current;
      if (ch) {
        try {
          ch.postMessage({
            kind: "starter-prefs:patch",
            workspaceId,
            patch,
          } satisfies PreferencesBroadcastMessage);
        } catch (err) {
          emitPreferencesTelemetry({
            kind: "prefs.broadcast_dropped",
            severity: "warn",
            patch: patch as Readonly<Record<string, unknown>>,
            reason: err instanceof Error ? err.message : String(err),
          });
        }
      }
    },
  });

  // Keep `setPreferences` stable across mutation state transitions.
  // react-query rebuilds the `mutation` object every time the internal
  // state (idle → pending → success) flips, so a naive
  // `useCallback(..., [mutation])` would change `setPreferences` two
  // or three times per flip — that ripples into the memoised context
  // value and breaks the Stage-7 render budget (one re-render per
  // consumer per `setPreferences` call). Funnel through a ref so the
  // callback identity is fixed for the provider's lifetime.
  const mutateRef = useRef(mutation.mutateAsync);
  useEffect(() => {
    mutateRef.current = mutation.mutateAsync;
  }, [mutation.mutateAsync]);
  const setPreferences = useCallback(
    (patch: PreferencesPatch) => mutateRef.current(patch),
    [],
  );

  const value = useMemo<PreferencesContextValue>(
    () => ({
      preferences: query.data ?? null,
      isLoading: query.isLoading,
      error: query.error,
      setPreferences,
    }),
    [query.data, query.isLoading, query.error, setPreferences],
  );

  // Side-effect: keep `<html lang>` (and `<html dir>` for RTL
  // languages) in sync with the resolved language so screen readers,
  // browser hyphenation, spell-check and font fallback all use the
  // right rules. The provider owns this so every consumer gets it
  // for free; teams that forget to wire it ship a broken a11y story
  // and do not notice.
  const language = query.data?.language;
  useEffect(() => {
    if (typeof document === "undefined" || !language) return;
    document.documentElement.lang = language;
    // RTL-list per `examples/notes/user-pref.md` Out-of-scope note.
    // No catalogs ship here, but setting `dir` keeps the chrome
    // correct the day one is added.
    const primary = language.split("-")[0]?.toLowerCase();
    const rtl = primary === "ar" || primary === "he" || primary === "fa" || primary === "ur";
    document.documentElement.dir = rtl ? "rtl" : "ltr";
  }, [language]);

  // A11y aria-live announcer (Stage-7 cross-cut). When the language
  // changes after the initial load, render a polite announcement so
  // screen-reader users get told the page changed language.
  // Translated via `Intl.DisplayNames` against the new language —
  // `"Idioma cambiado a Español"` for `es`. Skipped on the first
  // render (no announcement before the user has done anything).
  const [announcement, setAnnouncement] = useState<string>("");
  const previousLanguageRef = useRef<string | null>(null);
  useEffect(() => {
    if (!language) return;
    const prev = previousLanguageRef.current;
    previousLanguageRef.current = language;
    if (prev === null || prev === language) return;
    setAnnouncement(buildLanguageAnnouncement(language));
  }, [language]);

  // Loading contract — children only mount once `preferences` is
  // non-null. This guarantees the formatter hooks added in Stage 3
  // never run against `undefined` prefs.
  const ready = query.data != null;

  return (
    <PreferencesContext.Provider value={value}>
      {ready ? children : fallback}
      {/* aria-live announcer — visually hidden, screen-reader only.
       *  Polite so it does not interrupt other speech; cleared
       *  between flips so consecutive identical messages reset. */}
      <div
        data-testid="prefs-language-announcement"
        aria-live="polite"
        aria-atomic="true"
        style={SR_ONLY_STYLE}
      >
        {announcement}
      </div>
    </PreferencesContext.Provider>
  );
}

/** Visually hidden, screen-reader-only style. Inline because the
 *  provider must not depend on a global stylesheet. */
const SR_ONLY_STYLE = {
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
function buildLanguageAnnouncement(language: string): string {
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
const LANG_CHANGED_PHRASE: Record<string, string> = {
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

/** Read the preferences context. Throws if called outside a
 * `<PreferencesProvider>`. */
export function usePreferences(): PreferencesContextValue {
  const ctx = useContext(PreferencesContext);
  if (!ctx) throw new Error("usePreferences must be called inside <PreferencesProvider>");
  return ctx;
}

// ---------------------------------------------------------------------
// HTTP — minimal direct fetches via StarterClient. We do not depend on
// a generated endpoint module because /v1/me/preferences ships with
// starter-prefs, which sits outside the codegen pipeline today. The
// shape is hand-mirrored in `types.ts`.
// ---------------------------------------------------------------------

async function fetchMyPreferences(
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

async function patchMyPreferences(
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
