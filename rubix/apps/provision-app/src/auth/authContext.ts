import { createContext, useContext } from 'react'
import type { AuthUser, PingResult } from '../transport'

// Auth context + hook, separated from the provider component so the provider
// module exports only a component (Fast Refresh + one-concept-per-file).
export interface AuthState {
  user: AuthUser | null
  ready: boolean // initial me() check finished
  busy: boolean
  error: string | null
  transportKind: 'tauri' | 'web'
  /** Pre-login reachability check; never throws, result is the verdict. */
  ping: (baseUrl: string) => Promise<PingResult>
  login: (baseUrl: string, email: string, password: string) => Promise<void>
  logout: () => Promise<void>
}

export const AuthContext = createContext<AuthState | null>(null)

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}
