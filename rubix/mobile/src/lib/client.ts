// lib/client.ts — per-connection client factories.
//
// One `StarterClient` and one `RubixClient` per active connection. They
// are constructed fresh on every `setActiveId(...)` so the bearer header
// from the previous connection cannot leak. APP-SHELL.md §Provider stack.

import { StarterClient } from '@nube/starter-client-ts';
import { RubixClient } from '@nube/rubix-client-ts';

export const RUBIX_MOBILE_UA = `rubix-mobile/0.0.1`;

/**
 * Build a `RubixClient` (which wraps a `StarterClient`) pointed at a
 * specific server. The bearer is left unset — the auth strategy installs
 * it later via `client.starter.headers['Authorization']`.
 */
export function makeRubixClient(baseUrl: string): RubixClient {
  const starter = new StarterClient({
    baseUrl,
    apiPrefix: '/api/v1',
    headers: {
      'User-Agent': RUBIX_MOBILE_UA,
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
  });
  return new RubixClient(starter);
}
