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
  type ReactNode,
} from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { StarterClient } from "@nube/starter-client-ts";

import { starterQueryKey } from "../query/index.js";
import type { PreferencesPatch, ResolvedPreferences } from "./types.js";

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

const PreferencesContext = createContext<PreferencesContextValue | undefined>(undefined);

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

  const query = useQuery<ResolvedPreferences>({
    queryKey,
    queryFn: () => fetchMyPreferences(client, workspaceId),
  });

  const mutation = useMutation({
    mutationFn: (patch: PreferencesPatch) => patchMyPreferences(client, workspaceId, patch),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey });
    },
  });

  const setPreferences = useCallback(
    async (patch: PreferencesPatch) => {
      await mutation.mutateAsync(patch);
    },
    [mutation],
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

  // Side-effect: keep `<html lang>` in sync with the resolved
  // language so screen readers, browser hyphenation, spell-check and
  // font fallback all use the right rules. The provider owns this so
  // every consumer gets it for free; teams that forget to wire it
  // ship a broken a11y story and do not notice.
  const language = query.data?.language;
  useEffect(() => {
    if (typeof document === "undefined" || !language) return;
    document.documentElement.lang = language;
  }, [language]);

  // Loading contract — children only mount once `preferences` is
  // non-null. This guarantees the formatter hooks added in Stage 3
  // never run against `undefined` prefs.
  const ready = query.data != null;

  return (
    <PreferencesContext.Provider value={value}>
      {ready ? children : fallback}
    </PreferencesContext.Provider>
  );
}

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
