// Notes host wiring for the prefs + i18n surface.
//
// `<PrefsHostShell>` is the single mount point the rest of the app
// (and Stage 4's federated `HelloPanel`) reads from. It composes the
// three providers in the exact order the SCOPE requires:
//
//   <QueryClientProvider>          ← react-query for prefs/i18n hooks
//     <PreferencesProvider>        ← /v1/me/preferences, sets <html lang>
//       <IntlProvider>             ← /v1/i18n/manifest + catalog
//         {children}
//
// IntlProvider sits *inside* PreferencesProvider so it reads
// `prefs.language` directly; PreferencesProvider sits inside the
// caller's QueryClientProvider so both providers share the same
// react-query cache namespace (`["starter", "preferences", …]`).
//
// `<PrefsProbe>` is the smoke-test fixture: it renders one date and
// one temperature against the resolved prefs. The host mounts it in
// the header so an operator can sanity-check their locale at a
// glance, and `prefs-host.test.tsx` mounts it under a stub fetch to
// assert the en-AU + BBQ-°F path is correctly wired end-to-end.

import { useMemo, type ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { StarterClient } from "@nube/starter-client-ts";
import {
  PreferencesProvider,
  formatDate,
  formatQuantity,
  usePreferences,
} from "@nube/starter-ui-core/preferences";
import { IntlProvider } from "@nube/starter-ui-core/i18n";

export interface PrefsHostShellProps {
  client: StarterClient;
  /** Optional override so callers can share a QueryClient with the
   * rest of the app. Tests pass a no-retry client. */
  queryClient?: QueryClient;
  /** Skeleton shown while `/v1/me/preferences` is in flight. */
  fallback?: ReactNode;
  children: ReactNode;
}

/** Wires `<PreferencesProvider>` + `<IntlProvider>` into the notes
 * host. Children mount only after prefs resolve (loading contract). */
export function PrefsHostShell({
  client,
  queryClient,
  fallback,
  children,
}: PrefsHostShellProps) {
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
      <PreferencesProvider client={client} fallback={fallback}>
        <IntlProvider client={client}>{children}</IntlProvider>
      </PreferencesProvider>
    </QueryClientProvider>
  );
}

/** Stage-1 fixture probe: renders one date + one temperature against
 * the resolved prefs. Lives in the host so the test exercises the
 * real provider tree, not a mock. */
export function PrefsProbe({
  timestampMs,
  celsius,
}: {
  /** Fixture timestamp in UNIX millis. Defaults to 2026-04-22 12:00 UTC
   * (`22/04/2026` in `Australia/Brisbane`). */
  timestampMs?: number;
  /** Fixture temperature in canonical °C. Default 22.444°C ≈ 72.4°F. */
  celsius?: number;
} = {}) {
  const { preferences } = usePreferences();
  if (!preferences) return null;
  const ts = timestampMs ?? Date.UTC(2026, 3, 22, 12, 0, 0);
  const c = celsius ?? 22.444;
  return (
    <span
      data-testid="prefs-probe"
      style={{
        fontSize: "0.75rem",
        color: "var(--muted-foreground)",
        whiteSpace: "nowrap",
      }}
    >
      <span data-testid="prefs-probe-date">{formatDate(ts, preferences)}</span>
      {" · "}
      <span data-testid="prefs-probe-temp">
        {formatQuantity(c, "temperature", "celsius", preferences, {
          maximumFractionDigits: 1,
        })}
      </span>
    </span>
  );
}
