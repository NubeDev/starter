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
//
// Web-only side-effects living in this file:
//   * `document.documentElement.lang` / `.dir` writes (a11y / RTL).
//   * The aria-live announcer (`<div>` JSX).
//
// Everything DOM-free — the context, hook, constants, HTTP helpers,
// `buildLanguageAnnouncement`, `isRtlLanguage`, the SR_ONLY_STYLE —
// lives in `./provider-core.ts` so React-Native consumers (Hermes)
// can import the same surface without dragging the DOM bits in.

import {
  useCallback,
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
import {
  DEFAULT_WORKSPACE,
  PREFERENCES_BROADCAST_CHANNEL,
  PreferencesContext,
  type PreferencesBroadcastMessage,
  SR_ONLY_STYLE,
  buildLanguageAnnouncement,
  fetchMyPreferences,
  isRtlLanguage,
  patchMyPreferences,
} from "./provider-core.js";

// Backwards-compat re-exports — `./index.ts` and external consumers
// historically import these from `./provider.js`. Keep the surface
// stable; new RN-targeted callers can reach them via `./provider-core.js`.
export {
  DEFAULT_WORKSPACE,
  PREFERENCES_BROADCAST_CHANNEL,
  PreferencesContext,
  usePreferences,
} from "./provider-core.js";
export type {
  PreferencesBroadcastMessage,
  PreferencesContextValue,
} from "./provider-core.js";

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

  const value = useMemo(
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
  // and do not notice. WEB-ONLY — RN callers apply the same rule via
  // `isRtlLanguage()` from `./provider-core.js` against `I18nManager`.
  const language = query.data?.language;
  useEffect(() => {
    if (typeof document === "undefined" || !language) return;
    document.documentElement.lang = language;
    document.documentElement.dir = isRtlLanguage(language) ? "rtl" : "ltr";
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
