import type { AuthUser, PingResult, QueueItem, Transport } from './transport'

// Web/fetch implementation — talks to rubix-agent REST directly when NOT running
// under Tauri. Carries the session cookie (credentials:'include') + the CSRF
// token header on mutating calls. Read dedup + epoch-invalidation mirror the
// extension's api.ts so a read after a write can never join a stale in-flight
// request (the ~100ms read-after-write race).

const BASE_KEY = 'rbx.provision.baseUrl'
const QUEUE_KEY = 'rbx.provision.queue'

// Persist the agent base URL so reloads keep talking to the same host.
function readBaseUrl(): string {
  try {
    return localStorage.getItem(BASE_KEY) ?? ''
  } catch {
    return ''
  }
}
function writeBaseUrl(url: string) {
  try {
    localStorage.setItem(BASE_KEY, url)
  } catch {
    /* ignore */
  }
}

export function createWebTransport(): Transport {
  let baseUrl = readBaseUrl()
  let csrfToken = ''

  // Coalesce concurrent identical reads onto one in-flight promise; the epoch
  // is part of the key so a post-write read can't reuse a pre-write request.
  const inFlight = new Map<string, Promise<unknown>>()
  let readEpoch = 0

  function invalidateReads() {
    readEpoch += 1
    inFlight.clear()
  }

  function url(path: string): string {
    // Prefer SAME-ORIGIN /api so the browser keeps the agent's session
    // cookie. The cookie is `SameSite=Lax` — a browser will not send it on a
    // cross-site fetch, so calling the agent's absolute origin (e.g.
    // 127.0.0.1:8088) from a page on localhost:1421 authenticates login but
    // then drops the cookie on every tool call ("no caller identity / system
    // frame" → 403). In dev, Vite proxies /api → the agent (vite.config.ts);
    // in prod the app is served same-origin by the agent. Only fall back to
    // an absolute base if one is set AND it already matches the page origin.
    const root = baseUrl.replace(/\/+$/, '')
    const sameOrigin =
      typeof window !== 'undefined' && root && root === window.location.origin
    if (sameOrigin) return `${root}${path}`
    return path
  }

  async function request<T>(path: string, init: RequestInit, isMutation: boolean): Promise<T> {
    const headers = new Headers(init.headers)
    headers.set('accept', 'application/json')
    if (init.body) headers.set('content-type', 'application/json')
    if (isMutation && csrfToken) headers.set('X-CSRF-Token', csrfToken)

    const res = await fetch(url(path), {
      ...init,
      headers,
      credentials: 'include',
    })

    // Capture a freshly-minted CSRF token if the agent rotates it via header.
    const rotated = res.headers.get('X-CSRF-Token')
    if (rotated) csrfToken = rotated

    const text = await res.text()
    let parsed: unknown
    try {
      parsed = text ? JSON.parse(text) : undefined
    } catch {
      parsed = text
    }
    if (!res.ok) {
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
      // `/healthz` lives at the agent host ROOT, not under `/api/v1`.
      // Same-origin/prod: hit `${root}/healthz`. In browser dev the Vite
      // proxy only forwards `/api`, so a bare `/healthz` won't reach the
      // agent — but the web build is not the mobile target this is for.
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
      writeBaseUrl(nextBase)
      const out = await request<{ user?: AuthUser; csrf_token?: string } & AuthUser>(
        '/api/v1/auth/login',
        { method: 'POST', body: JSON.stringify({ email, password }) },
        true,
      )
      if (out.csrf_token) csrfToken = out.csrf_token
      invalidateReads()
      return out.user ?? (out as AuthUser)
    },

    async me() {
      if (!baseUrl) return null
      try {
        const out = await request<{ user?: AuthUser; csrf_token?: string } & AuthUser>(
          '/api/v1/auth/me',
          { method: 'GET' },
          false,
        )
        if (out.csrf_token) csrfToken = out.csrf_token
        return out.user ?? (out as AuthUser)
      } catch {
        return null
      }
    },

    async logout() {
      try {
        await request<void>('/api/v1/auth/logout', { method: 'POST' }, true)
      } finally {
        csrfToken = ''
        invalidateReads()
      }
    },

    dispatch<T>(toolId: string, params: unknown, opts?: { fresh?: boolean }) {
      const body = JSON.stringify(params ?? {})
      const key = `${toolId}::${body}::e${readEpoch}`
      if (!opts?.fresh) {
        const existing = inFlight.get(key)
        if (existing) return existing as Promise<T>
      }
      const p = request<T>(
        `/api/v1/tools/${toolId}`,
        { method: 'POST', body },
        // a tool call is a potential mutation; send CSRF defensively
        true,
      ).finally(() => {
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
