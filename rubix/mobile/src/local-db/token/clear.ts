// token/clear.ts — verb. Logout path.

import type { SecureTokenStore } from './contract';

export function clearToken(
  store: SecureTokenStore,
  connectionId: string,
): Promise<void> {
  return store.clear(connectionId);
}
