// `AuthProvider` — single source of truth for "who is the current
// user?" in a starter-powered React app.
//
// Backed by a TanStack Query against `starter.me()`. The query
// result drives three states for descendants:
//
//   - loading  → `me` is in flight, no decision yet.
//   - 401      → unauthenticated. Provider renders the
//                `unauthenticatedSlot` (typically the /login page).
//   - success  → authenticated. Children render normally and
//                `useAuth()` returns `{ user, isAuthenticated: true }`.
//
// `login` mutation calls `starter.login` then invalidates the `me`
// query so the success branch flips automatically. `logout` calls
// `starter.logout` then clears the React Query cache so stale
// per-user data does not leak across sessions.
//
// We intentionally keep the auth context tiny — components that need
// fine-grained access (mutation pending state, last error) should
// reach for the underlying hooks directly rather than us re-exposing
// every knob.

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  StarterError,
  type LoginRequest,
  type MeResponse,
} from "@nube/starter-client-ts";

import { useStarterClient } from "./starter-client-provider.js";

/** Stable query key for the `me` query — also used elsewhere to invalidate it. */
export const ME_QUERY_KEY = ["starter", "auth", "me"] as const;

export interface AuthContextValue {
  /** Current user, or `null` when not authenticated / still loading. */
  user: MeResponse | null;
  /** `true` only when `me` has resolved successfully. */
  isAuthenticated: boolean;
  /** Authenticate with credentials. Resolves on success, throws otherwise. */
  login(request: LoginRequest): Promise<void>;
  /** Drop the session cookie and clear cached per-user data. */
  logout(): Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export interface AuthProviderProps {
  /**
   * Rendered when `me` fails with 401 (i.e. no session). Typically
   * the /login page. Receiving it as a slot keeps this provider
   * router-agnostic — the app decides how to render the login UX.
   */
  unauthenticatedSlot: ReactNode;
  /** Rendered while the initial `me` query is in flight. */
  loadingSlot?: ReactNode;
  children: ReactNode;
}

/**
 * Provide authentication state to descendants.
 *
 * Mount inside `QueryProvider` (it depends on a `QueryClient`) and
 * `StarterClientProvider` (it depends on a `StarterClient`).
 */
export function AuthProvider(props: AuthProviderProps) {
  const starter = useStarterClient();
  const queryClient = useQueryClient();

  const meQuery = useQuery<MeResponse, StarterError>({
    queryKey: ME_QUERY_KEY,
    queryFn: () => starter.me(),
    // `me` is small and central — keep it fresh on tab focus, but
    // don't thrash the server: rely on the 30s staleTime from
    // `QueryProvider` defaults.
  });

  const loginMutation = useMutation({
    mutationFn: (request: LoginRequest) => starter.login(request),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ME_QUERY_KEY });
    },
  });

  const logoutMutation = useMutation({
    mutationFn: () => starter.logout(),
    onSuccess: () => {
      // Hard reset rather than invalidate: a logged-out user must
      // not see another user's cached pages on next login.
      queryClient.clear();
    },
  });

  const login = useCallback(
    async (request: LoginRequest) => {
      await loginMutation.mutateAsync(request);
    },
    [loginMutation],
  );

  const logout = useCallback(async () => {
    await logoutMutation.mutateAsync();
  }, [logoutMutation]);

  const value = useMemo<AuthContextValue>(
    () => ({
      user: meQuery.data ?? null,
      isAuthenticated: meQuery.isSuccess && meQuery.data != null,
      login,
      logout,
    }),
    [meQuery.data, meQuery.isSuccess, login, logout],
  );

  // Decide which slot to render. We split the unauthenticated check
  // off the StarterError 401 specifically — other errors (network,
  // 500) should not silently flip the UI to "please log in".
  const is401 =
    meQuery.isError &&
    meQuery.error instanceof StarterError &&
    meQuery.error.status === 401;

  let body: ReactNode;
  if (meQuery.isPending) {
    body = props.loadingSlot ?? null;
  } else if (is401) {
    body = props.unauthenticatedSlot;
  } else {
    body = props.children;
  }

  return <AuthContext.Provider value={value}>{body}</AuthContext.Provider>;
}

/**
 * Read auth state. Throws if no `AuthProvider` is mounted — that's
 * a programming error, not a runtime condition.
 */
export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error(
      "useAuth() called outside <AuthProvider>. Mount AuthProvider " +
        "inside StarterClientProvider + QueryProvider near your app root.",
    );
  }
  return ctx;
}
