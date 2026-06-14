import { createContext, useContext, type ReactNode } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { StarterError } from "@nube/starter-client-ts";

import { getMe } from "@/api/me/get";
import { login as loginRequest, logout as logoutRequest } from "@/auth/login";
import { ME_QUERY_KEY } from "@/auth/usePrincipal";
import type { LoginRequest } from "@/auth/login";
import type { MeResponse } from "@/api/types";

// Nexus's own auth provider. It mirrors `@nube/starter-client-react`'s
// `AuthProvider` (probe `me` on mount, swap to the login slot on 401) but
// hits nexus-api's paths: the principal at `/api/v1/me` and cookie-session
// login at the server root `/auth/*` — the starter provider assumes
// `/api/v1/auth/*`, which 404s here.
interface AuthContextValue {
  user: MeResponse | null;
  isAuthenticated: boolean;
  login: (request: LoginRequest) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({
  unauthenticatedSlot,
  loadingSlot,
  children,
}: {
  unauthenticatedSlot: ReactNode;
  loadingSlot?: ReactNode;
  children: ReactNode;
}) {
  const client = useStarterClient();
  const queryClient = useQueryClient();

  const meQuery = useQuery<MeResponse, StarterError>({
    queryKey: ME_QUERY_KEY,
    queryFn: () => getMe(client),
    retry: false,
    staleTime: 5 * 60_000,
  });

  const loginMutation = useMutation({
    mutationFn: (request: LoginRequest) => loginRequest(client, request),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ME_QUERY_KEY }),
  });

  const logoutMutation = useMutation({
    mutationFn: () => logoutRequest(client),
    // Hard reset, not invalidate: a logged-out user must not see another
    // user's cached pages on next login.
    onSuccess: () => queryClient.clear(),
  });

  const value: AuthContextValue = {
    user: meQuery.data ?? null,
    isAuthenticated: meQuery.isSuccess && meQuery.data != null,
    login: async (request) => {
      await loginMutation.mutateAsync(request);
    },
    logout: async () => {
      await logoutMutation.mutateAsync();
    },
  };

  // A 401 means unauthenticated → render the login slot; any other error
  // is a real failure surfaced by the query, not a reason to show login.
  const isUnauthenticated =
    meQuery.isError && StarterError.is(meQuery.error, 401);

  let body: ReactNode;
  if (meQuery.isPending) body = loadingSlot ?? null;
  else if (isUnauthenticated || !value.isAuthenticated) body = unauthenticatedSlot;
  else body = children;

  return <AuthContext.Provider value={value}>{body}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within the Nexus AuthProvider");
  return ctx;
}
