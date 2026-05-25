// token/get.ts — verb. Read the bearer for a connection. Trivial wrapper
// preserved for symmetry with the other verb files in this folder.

import type { SecureTokenStore } from './contract';

export function getToken(
  store: SecureTokenStore,
  connectionId: string,
): Promise<string | null> {
  return store.get(connectionId);
}
