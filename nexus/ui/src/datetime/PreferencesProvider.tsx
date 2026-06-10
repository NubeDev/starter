import { useCallback, useMemo, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { starterQueryKey } from "@nube/starter-ui-core/query";
import {
  PreferencesContext,
  type PreferencesContextValue,
} from "@nube/starter-ui-core/preferences";
import type { PreferencesPatch } from "@nube/starter-ui-core/preferences";

import { getNexusClient } from "@/api/client";
import { getMyPreferences, patchMyPreferences } from "@/api/me/preferences";

// Nexus-native preferences provider — the missing link that finally
// connects the WS-11 backend (`GET`/`PATCH /api/v1/me/preferences` +
// the `Accept-Units` middleware) to the already-built UI seam.
//
// Why not the stock `@nube/starter-ui-core` `<PreferencesProvider>`?
// Its built-in fetcher hits `${baseUrl}/v1/me/preferences?org=<ws>` —
// the wrong path for nexus (no `/api` prefix) and, worse, a spoofable
// `?org=` selector that nexus deliberately rejects (isolation is
// route-pinned from the principal, not a query param). So we mount our
// OWN provider that fills the SAME `PreferencesContext` and uses the
// SAME react-query key, but fetches/patches through the nexus binding
// (`api/me/preferences.ts`, which rides `fetchJson` + CSRF like every
// other nexus call). `useDateTime()`'s context branch and any
// `usePreferences()` consumer (incl. federated extensions reading the
// host singleton) light up with backend-resolved prefs — zero
// call-site changes.
//
// Until this provider resolves prefs, `preferences` is `null` and
// `useDateTime()` cleanly falls back to the local per-device
// `datetime/store.ts` (device locale + region quick-set), so the app
// never blocks on the network and degrades gracefully when unauthed.

// Match the Rust resolver's default scope so the cache key agrees with
// the platform singleton (a future federated consumer keyed the stock
// way still shares this entry).
const WORKSPACE = "@starter/default";

export function PreferencesProvider({ children }: { children: ReactNode }) {
  const client = getNexusClient();
  const queryClient = useQueryClient();
  const queryKey = useMemo(() => starterQueryKey("preferences", WORKSPACE), []);

  const query = useQuery({
    queryKey,
    queryFn: () => getMyPreferences(client),
    // Prefs change rarely and drive global formatting — keep them warm.
    staleTime: 5 * 60_000,
  });

  const mutation = useMutation({
    mutationFn: (patch: PreferencesPatch) => patchMyPreferences(client, patch),
    onSuccess: (resolved) => {
      // The PATCH returns the freshly resolved view — seed the cache so
      // every consumer re-renders immediately, then revalidate.
      queryClient.setQueryData(queryKey, resolved);
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

  return (
    <PreferencesContext.Provider value={value}>
      {children}
    </PreferencesContext.Provider>
  );
}
