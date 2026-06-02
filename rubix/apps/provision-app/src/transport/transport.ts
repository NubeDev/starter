// `transport.ts` — the single interface the whole app talks to. Two
// implementations sit behind it (Tauri invoke vs web fetch), chosen at runtime
// in `chooseTransport`. The bc API layer (api/bc.ts) calls `dispatch` only;
// nothing else in the app touches fetch or invoke directly.

export interface AuthUser {
  email: string
  [k: string]: unknown
}

export interface QueueItem {
  id: string
  tool_id: string
  params: unknown
  enqueued_at: number
}

/** Outcome of a pre-login connectivity probe against `{base}/healthz`. */
export interface PingResult {
  ok: boolean
  latency_ms: number | null
  message: string
}

export interface Transport {
  /**
   * No-auth liveness probe of `{baseUrl}/healthz`. Lets the UI confirm
   * the agent is reachable (and tell that apart from bad credentials)
   * before attempting a login. Never throws on an unreachable host —
   * the failure is reported in `PingResult.ok`/`message`.
   */
  ping(baseUrl: string): Promise<PingResult>
  /**
   * The agent base URL this device last connected to, or '' if none. Lets the
   * Connect screen pre-fill the host the operator actually used instead of the
   * compiled-in default — survives logout (logout drops credentials, not the
   * remembered host). Synchronous: backed by localStorage on web. Optional so
   * a transport that can't read it synchronously (e.g. Tauri, whose URL lives
   * in the Rust core) may omit it; callers fall back to the default.
   */
  savedBaseUrl?(): string
  /**
   * Async counterpart of `savedBaseUrl` for transports whose remembered host
   * lives outside the JS layer. On Tauri the base_url is persisted by the Rust
   * core (keychain/store) and can only be read over `invoke`, so the Connect
   * screen hydrates it asynchronously after mount. Returns '' if none. Optional
   * for the same reason `savedBaseUrl` is — web can answer synchronously and
   * leaves this unset.
   */
  savedBaseUrlAsync?(): Promise<string>
  /** Sign in. Establishes the session (cookie on web, keychain on Tauri). */
  login(baseUrl: string, email: string, password: string): Promise<AuthUser>
  /** Current principal, or null if unauthenticated. */
  me(): Promise<AuthUser | null>
  logout(): Promise<void>
  /**
   * Invoke a tool by id. `fresh` skips read-dedup so a read after a write
   * never observes a coalesced pre-write result (the ~100ms race).
   */
  dispatch<T>(toolId: string, params: unknown, opts?: { fresh?: boolean }): Promise<T>

  /** Offline scan queue (degrades to a localStorage stub on web). */
  queueEnqueue(toolId: string, params: unknown): Promise<QueueItem>
  queueList(): Promise<ReadonlyArray<QueueItem>>
  queueFlush(): Promise<{ flushed: number; failed: number }>
  queueDrop(id: string): Promise<void>

  /** Which backend this instance speaks to — for diagnostics in the UI. */
  readonly kind: 'tauri' | 'web'
}

// True when running inside a Tauri webview. Tauri injects this global.
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}
