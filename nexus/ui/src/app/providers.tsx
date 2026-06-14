import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { StarterClientProvider } from "@nube/starter-client-react";
import { ExtensionHostProvider } from "@nube/starter-ext-ui";

import { getNexusClient } from "@/api/client";
import { getExtensionHost } from "@/extensions/host";
import { createNexusQueryClient } from "@/app/queryClient";
import { AuthProvider } from "@/auth/AuthProvider";
import { LoginRoute } from "@/auth/LoginRoute";
import { ExtensionAutoLoader } from "@/extensions/AutoLoader";
import { PreferencesProvider } from "@/datetime/PreferencesProvider";
import { ThemeProvider } from "@/theme";

// Provider nesting mirrors the canonical starter host (rubix/frontend):
// ThemeProvider (live OS dark/light sync) → QueryClient (shared
// singleton) → StarterClient (data ingress) → ExtensionHost (federation
// runtime) → AuthProvider (one app-root guard that swaps the whole tree
// for the login slot on 401). ThemeProvider is outermost so the theme
// follows the OS even on the login screen; the auth guard lives here so
// routed screens stay guard-free (F4). Inside the guard, PreferencesProvider
// resolves the caller's backend prefs (WS-11) so date/time/units render
// per org/user settings, not just the local per-device fallback.
const queryClient = createNexusQueryClient();

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider>
      <QueryClientProvider client={queryClient}>
        <StarterClientProvider client={getNexusClient()}>
          <ExtensionHostProvider host={getExtensionHost()}>
            <AuthProvider unauthenticatedSlot={<LoginRoute />}>
              {/* Mounted inside the auth guard so prefs fetch only with a
                  live session (avoids a 401 probe on the login screen) and
                  feed every `useDateTime()`/`usePreferences()` consumer the
                  backend-resolved tz/units/format (WS-11). */}
              <PreferencesProvider>
                <ExtensionAutoLoader />
                {children}
              </PreferencesProvider>
            </AuthProvider>
          </ExtensionHostProvider>
        </StarterClientProvider>
      </QueryClientProvider>
    </ThemeProvider>
  );
}
