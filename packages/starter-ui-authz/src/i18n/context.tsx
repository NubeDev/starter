// React context carrying the resolved `AuthzMessages` from
// `<AuthzAdmin>` (or a standalone `<AuthzI18nProvider>`) down to
// the panel components.

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  DEFAULT_AUTHZ_MESSAGES,
  mergeAuthzMessages,
  type AuthzMessages,
} from "./messages.js";

const AuthzI18nContext = createContext<AuthzMessages>(DEFAULT_AUTHZ_MESSAGES);

export interface AuthzI18nProviderProps {
  /** Partial override merged on top of `DEFAULT_AUTHZ_MESSAGES`. */
  value?: Partial<AuthzMessages>;
  children: ReactNode;
}

/** Provider — `<AuthzAdmin>` wraps its tree in this automatically. */
export function AuthzI18nProvider({ value, children }: AuthzI18nProviderProps) {
  const merged = useMemo(() => mergeAuthzMessages(value), [value]);
  return (
    <AuthzI18nContext.Provider value={merged}>
      {children}
    </AuthzI18nContext.Provider>
  );
}

/** Read the current `AuthzMessages`. Falls back to defaults outside
 * a provider. */
export function useAuthzMessages(): AuthzMessages {
  return useContext(AuthzI18nContext);
}
