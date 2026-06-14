import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { transport, type AuthUser } from '../transport'
import { useAppTheme } from '../theme/themeContext'
import { AuthContext, type AuthState } from './authContext'

// Tauri commands reject with the Rust `AppError` shape `{ kind, message }`,
// which is NOT an Error instance — so `e.message` alone falls back to a
// generic string and hides the real reason (e.g. "invalid credentials" vs
// "transport error: ... connection refused"). Dig the message out of either.
function errMessage(e: unknown, fallback: string): string {
  if (e instanceof Error) return e.message
  if (e && typeof e === 'object' && 'message' in e) {
    const m = (e as { message: unknown }).message
    if (typeof m === 'string' && m) return m
  }
  if (typeof e === 'string' && e) return e
  return fallback
}

// Session state for the whole app. Gates the UI until authenticated. The live
// connection status tints the app accent (online when authed, offline when not).
export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null)
  const [ready, setReady] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { setStatus } = useAppTheme()

  // Restore any existing session on boot.
  useEffect(() => {
    let alive = true
    transport
      .me()
      .then((u) => {
        if (!alive) return
        setUser(u)
        setStatus(u ? 'online' : 'offline')
      })
      .finally(() => alive && setReady(true))
    return () => {
      alive = false
    }
  }, [setStatus])

  const ping = useCallback(async (baseUrl: string) => {
    setError(null)
    return transport.ping(baseUrl)
  }, [])

  const login = useCallback(
    async (baseUrl: string, email: string, password: string) => {
      setBusy(true)
      setError(null)
      setStatus('pairing')
      try {
        const u = await transport.login(baseUrl, email, password)
        setUser(u)
        setStatus('online')
      } catch (e) {
        setStatus('fault')
        setError(errMessage(e, 'Login failed'))
        throw e
      } finally {
        setBusy(false)
      }
    },
    [setStatus],
  )

  const logout = useCallback(async () => {
    await transport.logout()
    setUser(null)
    setStatus('offline')
  }, [setStatus])

  const value = useMemo<AuthState>(
    () => ({ user, ready, busy, error, transportKind: transport.kind, ping, login, logout }),
    [user, ready, busy, error, ping, login, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
