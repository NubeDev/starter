// token/put.ts — verb. Write a freshly-issued bearer.

import type { SecureTokenStore } from './contract';

export function putToken(
  store: SecureTokenStore,
  connectionId: string,
  token: string,
): Promise<void> {
  return store.put(connectionId, token);
}
