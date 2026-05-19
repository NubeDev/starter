// `<AuthProvider>` + `useAuth()`. Holds the StarterClient, the current
// AuthStrategy, and the current `MeResponse | null`. State managed via
// `useState` rather than zustand to keep the hook surface tree-local
// (each provider mount has its own world).

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { MeResponse, StarterClient } from "@nube/starter-client-ts";

import type { AuthStrategy, LoginInput } from "./strategy.js";

export type AuthStatus = "loading" | "authenticated" | "unauthenticated";

export interface AuthContextValue {
  status: AuthStatus;
  user: MeResponse | null;
  login: (input: LoginInput) => Promise<void>;
  logout: () => Promise<void>;
  /** Re-run the `me()` probe. Useful after an out-of-band token change. */
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export interface AuthProviderProps {
  client: StarterClient;
  strategy: AuthStrategy;
  children: ReactNode;
}

export function AuthProvider({ client, strategy, children }: AuthProviderProps) {
  const [user, setUser] = useState<MeResponse | null>(null);
  const [status, setStatus] = useState<AuthStatus>("loading");
  // Ref so refresh() always sees the latest strategy without re-binding.
  const strategyRef = useRef(strategy);
  strategyRef.current = strategy;

  const refresh = useCallback(async () => {
    const me = await strategyRef.current.load(client);
    setUser(me);
    setStatus(me ? "authenticated" : "unauthenticated");
  }, [client]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const login = useCallback(
    async (input: LoginInput) => {
      const me = await strategyRef.current.login(client, input);
      setUser(me);
      setStatus("authenticated");
    },
    [client],
  );

  const logout = useCallback(async () => {
    await strategyRef.current.logout(client);
    setUser(null);
    setStatus("unauthenticated");
  }, [client]);

  const value = useMemo<AuthContextValue>(
    () => ({ status, user, login, logout, refresh }),
    [status, user, login, logout, refresh],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be called inside <AuthProvider>");
  return ctx;
}
