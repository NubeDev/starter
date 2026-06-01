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

export interface Transport {
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
