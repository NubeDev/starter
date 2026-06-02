import type { AuthUser, PingResult, QueueItem, Transport } from './transport'

// Web/fetch implementation — talks to the rubix-agent REST API directly over
// the network (LAN or internet), authenticating with a Bearer token.
//
// Why Bearer and not the session cookie: the agent's `starter_session` cookie
// is `SameSite=Lax`, so a browser will NOT send it on a cross-site request. As
// soon as the app and the agent live on different origins — which is the whole
// point here, the phone talking to a remote agent — cookie auth is dead on
// arrival (login succeeds, every later call drops the cookie → 401/403).
//
// The agent is built for exactly this: `POST /auth/token` mints a `sak_…`
// Bearer (the documented "cookie-less counterpart of login — mobile, native
// desktop, and CLI sign-in", 30-day TTL), and every protected route accepts
// `Authorization: Bearer sak_…` with no cookie. So we mint a token at login,
// persist it, and send it on every call against the absolute agent URL. No
// proxy, no cookie, no CSRF (CSRF only guards cookie auth) — works over the air.

const BASE_KEY = 'rbx.provision.baseUrl'
const TOKEN_KEY = 'rbx.provision.token'
const QUEUE_KEY = 'rbx.provision.queue'

function readStored(key: string): string {
  try {
    return localStorage.getItem(key) ?? ''
  } catch {
    return ''
  }
}
function writeStored(key: string, value: string) {
  try {
    if (value) localStorage.setItem(key, value)
    else localStorage.removeItem(key)
  } catch {
    /* ignore */
  }
}

export function createWebTransport(): Transport {
  // Persist base URL + token so a reload keeps talking to the same agent
  // without forcing the operator to re-enter credentials.
  let baseUrl = readStored(BASE_KEY)
  let token = readStored(TOKEN_KEY)

  // Coalesce concurrent identical reads onto one in-flight promise; the epoch
  // is part of the key so a post-write read can't reuse a pre-write request.
  const inFlight = new Map<string, Promise<unknown>>()
  let readEpoch = 0

  function invalidateReads() {
    readEpoch += 1
    inFlight.clear()
  }

  // Absolute URL against the configured agent. The agent serves the API under
  // `/api/v1`; healthz lives at the host root.
  function url(path: string): string {
    const root = baseUrl.replace(/\/+$/, '')
    return `${root}${path}`
  }

  async function request<T>(path: string, init: RequestInit): Promise<T> {
    const headers = new Headers(init.headers)
    headers.set('accept', 'application/json')
    if (init.body) headers.set('content-type', 'application/json')
    if (token) headers.set('authorization', `Bearer ${token}`)

    const res = await fetch(url(path), { ...init, headers })

    const text = await res.text()
    let parsed: unknown
    try {
      parsed = text ? JSON.parse(text) : undefined
    } catch {
      parsed = text
    }
    if (!res.ok) {
      // A stale/expired token (30-day TTL) shows up as 401 — drop it so the
      // app falls back to the Connect screen instead of looping on a dead token.
      if (res.status === 401) {
        token = ''
        writeStored(TOKEN_KEY, '')
      }
      const msg =
        parsed && typeof parsed === 'object' && parsed && 'error' in parsed
          ? String((parsed as { error: unknown }).error)
          : `HTTP ${res.status}`
      throw new Error(msg)
    }
    return parsed as T
  }

  return {
    kind: 'web',

    async ping(nextBase): Promise<PingResult> {
      // `/healthz` lives at the agent host ROOT, not under `/api/v1`. No auth.
      const root = nextBase.replace(/\/+$/, '')
      const target = root ? `${root}/healthz` : '/healthz'
      const ctrl = new AbortController()
      const timer = setTimeout(() => ctrl.abort(), 4000)
      const started = performance.now()
      try {
        const res = await fetch(target, { method: 'GET', signal: ctrl.signal })
        const latency = Math.round(performance.now() - started)
        if (res.ok) return { ok: true, latency_ms: latency, message: `reachable in ${latency} ms` }
        return { ok: false, latency_ms: null, message: `agent answered HTTP ${res.status} (host reachable)` }
      } catch (e) {
        const reason = e instanceof DOMException && e.name === 'AbortError' ? 'timed out' : String(e)
        return { ok: false, latency_ms: null, message: `cannot reach ${target}: ${reason}` }
      } finally {
        clearTimeout(timer)
      }
    },

    async login(nextBase, email, password) {
      baseUrl = nextBase
      writeStored(BASE_KEY, nextBase)
      // Mint a Bearer token. This is the cookie-less sign-in path; the response
      // carries the `sak_…` plaintext exactly once, so we persist it now.
      const out = await request<{ token: string; expires_at?: string }>('/api/v1/auth/token', {
        method: 'POST',
        body: JSON.stringify({ email, password }),
      })
      token = out.token
      writeStored(TOKEN_KEY, token)
      invalidateReads()
      // `/auth/token` returns only the token; fetch the principal for the UI.
      const user = await this.me()
      if (!user) throw new Error('login succeeded but identity lookup failed')
      return user
    },

    async me() {
      if (!baseUrl || !token) return null
      try {
        const out = await request<{ user?: AuthUser } & AuthUser>('/api/v1/auth/me', {
          method: 'GET',
        })
        return out.user ?? (out as AuthUser)
      } catch {
        return null
      }
    },

    async logout() {
      // Bearer tokens are stateless on the client side; clearing the stored
      // token signs this device out. (The token remains valid server-side until
      // its TTL — there is no per-token revoke route exposed here.)
      token = ''
      writeStored(TOKEN_KEY, '')
      invalidateReads()
    },

    dispatch<T>(toolId: string, params: unknown, opts?: { fresh?: boolean }) {
      const body = JSON.stringify(params ?? {})
      const key = `${toolId}::${body}::e${readEpoch}`
      if (!opts?.fresh) {
        const existing = inFlight.get(key)
        if (existing) return existing as Promise<T>
      }
      const p = request<T>(`/api/v1/tools/${toolId}`, { method: 'POST', body }).finally(() => {
        if (inFlight.get(key) === p) inFlight.delete(key)
      })
      if (!opts?.fresh) inFlight.set(key, p)
      return p
    },

    // ── Offline queue stub (localStorage) ──────────────────────────────────
    // On web there's no durable native queue, so we degrade to localStorage.
    // The Tauri build gets the real durable queue from the Rust core.
    async queueEnqueue(toolId, params) {
      const item: QueueItem = {
        id: `q_${readEpoch}_${Date.now().toString(36)}`,
        tool_id: toolId,
        params,
        enqueued_at: Date.now(),
      }
      const list = readQueue()
      writeQueue([...list, item])
      return item
    },
    async queueList() {
      return readQueue()
    },
    async queueFlush() {
      const list = readQueue()
      let flushed = 0
      let failed = 0
      const remaining: QueueItem[] = []
      for (const item of list) {
        try {
          await this.dispatch(item.tool_id, item.params, { fresh: true })
          flushed += 1
        } catch {
          failed += 1
          remaining.push(item)
        }
      }
      writeQueue(remaining)
      invalidateReads()
      return { flushed, failed }
    },
    async queueDrop(id) {
      writeQueue(readQueue().filter((q) => q.id !== id))
    },
  }
}

function readQueue(): QueueItem[] {
  try {
    const raw = localStorage.getItem(QUEUE_KEY)
    return raw ? (JSON.parse(raw) as QueueItem[]) : []
  } catch {
    return []
  }
}
function writeQueue(list: QueueItem[]) {
  try {
    localStorage.setItem(QUEUE_KEY, JSON.stringify(list))
  } catch {
    /* ignore */
  }
}
