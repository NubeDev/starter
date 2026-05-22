// flow-agent host wiring for the prefs + i18n surface. Mirrors the
// `examples/notes/frontend/src/prefs-host.tsx` shape so the
// three-provider sandwich (QueryClient → Preferences → Intl) is
// composed once and consumed by the rest of the SPA via the standard
// `usePreferences()` / `useTranslate()` hooks.
//
// flow-agent has no auth, so the preferences row is keyed on the
// backend-side `local-operator` principal injected by
// `with_anonymous_principal`. The default workspace
// (`"@starter/default"`) is used implicitly by the provider.

import { useMemo, type ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StarterClient } from "@nube/starter-client-ts";
import { PreferencesProvider } from "@nube/starter-ui-core/preferences";
import { IntlProvider } from "@nube/starter-ui-core/i18n";

export interface PrefsHostShellProps {
  /** Optional override so callers can share a QueryClient with the
   * rest of the app. Tests pass a no-retry client. */
  queryClient?: QueryClient;
  /** Skeleton shown while `/v1/me/preferences` is in flight. */
  fallback?: ReactNode;
  children: ReactNode;
}

/** Shared `StarterClient` for the whole SPA. Empty `baseUrl` means
 * "same origin" — vite proxies `/v1/*` to the flow-agent backend in
 * dev, and the static build is served by the same backend in prod. */
const starterClient = new StarterClient({ baseUrl: "" });

/** Wires `<PreferencesProvider>` + `<IntlProvider>` into flow-agent. */
export function PrefsHostShell({
  queryClient,
  fallback,
  children,
}: PrefsHostShellProps) {
  // The host already mounts a QueryClientProvider, but the prefs +
  // i18n hooks need react-query too. Reuse the caller's client when
  // provided; otherwise fall back to a no-retry instance scoped to
  // this provider tree.
  const qc = useMemo(
    () =>
      queryClient ??
      new QueryClient({
        defaultOptions: {
          queries: { retry: false, refetchOnWindowFocus: false },
        },
      }),
    [queryClient],
  );

  return (
    <QueryClientProvider client={qc}>
      <PreferencesProvider client={starterClient} fallback={fallback}>
        <IntlProvider client={starterClient}>{children}</IntlProvider>
      </PreferencesProvider>
    </QueryClientProvider>
  );
}
