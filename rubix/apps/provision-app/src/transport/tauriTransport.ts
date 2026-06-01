import { invoke } from '@tauri-apps/api/core'
import type { AuthUser, QueueItem, Transport } from './transport'

// Tauri implementation — calls the Rust core's commands (built by the sibling
// agent in src-tauri/). The Rust side owns the HTTP session, CSRF, and the
// durable offline queue, so this layer is a thin invoke() forwarder.
export function createTauriTransport(): Transport {
  return {
    kind: 'tauri',

    login(baseUrl, email, password) {
      return invoke<AuthUser>('auth_login', { baseUrl, email, password })
    },
    me() {
      return invoke<AuthUser | null>('auth_me')
    },
    logout() {
      return invoke<void>('auth_logout')
    },
    dispatch<T>(toolId: string, params: unknown, opts?: { fresh?: boolean }) {
      return invoke<T>('tool_dispatch', { toolId, params, fresh: opts?.fresh ?? false })
    },

    queueEnqueue(toolId, params) {
      return invoke<QueueItem>('queue_enqueue', { toolId, params })
    },
    queueList() {
      return invoke<ReadonlyArray<QueueItem>>('queue_list')
    },
    queueFlush() {
      return invoke<{ flushed: number; failed: number }>('queue_flush')
    },
    queueDrop(id) {
      return invoke<void>('queue_drop', { id })
    },
  }
}
