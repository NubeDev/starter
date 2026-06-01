// Re-export + the runtime chooser. The rest of the app imports `transport`
// from here and never knows which implementation it got.
import { isTauri, type Transport } from './transport'
import { createTauriTransport } from './tauriTransport'
import { createWebTransport } from './webTransport'

export type { Transport, AuthUser, QueueItem } from './transport'
export { isTauri } from './transport'

// Chosen once at module load: Tauri impl when inside the webview (the global
// `__TAURI_INTERNALS__` is present), otherwise the web/fetch impl.
export const transport: Transport = isTauri() ? createTauriTransport() : createWebTransport()
