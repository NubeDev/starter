// auth/install.ts — install / clear a bearer on a StarterClient.
//
// We do NOT use `tokenStrategy.login` from `@nube/starter-ui-core/auth`
// because that helper probes `/auth/me` after installing the header,
// and the rubix backend's `/auth/me` is cookie-only — it returns 401
// for bearer requests by design (the bearer surface is `/api/v1/tools/*`).
// So we hand-roll the in-memory install + secure-store mirror here.
//
// APP-SHELL.md §Strategy + token-issuance.md document this carveout.

import type { StarterClient } from '@nube/starter-client-ts';

import type { SecureTokenStore } from '../local-db/token/contract';

/** Install bearer on the in-memory client AND persist for cold-start. */
export async function installBearer(args: {
  client: StarterClient;
  secureStore: SecureTokenStore;
  connectionId: string;
  token: string;
}): Promise<void> {
  args.client.headers['Authorization'] = `Bearer ${args.token}`;
  await args.secureStore.put(args.connectionId, args.token);
}

/** Remove bearer from memory AND from secure-store. */
export async function clearBearer(args: {
  client: StarterClient;
  secureStore: SecureTokenStore;
  connectionId: string;
}): Promise<void> {
  delete args.client.headers['Authorization'];
  await args.secureStore.clear(args.connectionId);
}
