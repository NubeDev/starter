import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import {
  AuthProvider,
  StarterClientProvider,
} from "@nube/starter-client-react";
import { ExtensionHostProvider } from "@nube/starter-ext-ui";

import { getNexusClient } from "@/api/client";
import { getExtensionHost } from "@/extensions/host";
import { createNexusQueryClient } from "@/app/queryClient";
import { LoginRoute } from "@/auth/LoginRoute";
import { ExtensionAutoLoader } from "@/extensions/AutoLoader";

// Provider nesting mirrors the canonical starter host (rubix/frontend):
// QueryClient (shared singleton) → StarterClient (data ingress) →
// ExtensionHost (federation runtime) → AuthProvider (one app-root guard
// that swaps the whole tree for the login slot on 401). The auth guard
// lives here so routed screens stay guard-free (F4).
const queryClient = createNexusQueryClient();

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <StarterClientProvider client={getNexusClient()}>
        <ExtensionHostProvider host={getExtensionHost()}>
          <AuthProvider unauthenticatedSlot={<LoginRoute />}>
            <ExtensionAutoLoader />
            {children}
          </AuthProvider>
        </ExtensionHostProvider>
      </StarterClientProvider>
    </QueryClientProvider>
  );
}
