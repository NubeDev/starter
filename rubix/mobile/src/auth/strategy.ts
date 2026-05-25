// auth/strategy.ts — the two-step mobile login flow.
//
// Per APP-SHELL.md §Strategy: mobile cannot reuse the upstream
// `tokenStrategy.login` directly because it probes `/auth/me` after
// installing the header, and the rubix backend's `/auth/me` is cookie-
// only (bearer surface is `/api/v1/tools/*`). So the login is hand-
// rolled in two clear steps:
//
//   1. POST { email, password [, tenant_id] } to /api/v1/auth/token
//      (the route landed in `33ed0ca` — see
//      rubix/docs/design/auth/token-issuance.md). Returns
//      { token, expires_at, token_type }.
//   2. `installBearer(...)` writes the header on the in-memory
//      StarterClient and mirrors to expo-secure-store, keyed by
//      connection id so a cold start can rehydrate.

import type { StarterClient } from '@nube/starter-client-ts';

import type { SecureTokenStore } from '../local-db/token/contract';
import { installBearer } from './install';

/** Wire-shape of `POST /api/v1/auth/token`. Mirrors `TokenResponse` in
 *  `crates/starter-auth-users/src/routes/token.rs`. */
export interface IssuedToken {
  token: string;
  expires_at: string;
  token_type: 'Bearer';
}

/** Discriminated error envelope from the same route. */
export interface TokenError {
  status: number;
  body?: unknown;
  message?: string;
}

/**
 * Step 1 — credentials → bearer. Hits the new
 * `POST <baseUrl>/api/v1/auth/token` route directly via plain `fetch`
 * (the call happens before there is a token to install on the client,
 * and a hand-rolled fetch keeps the call site obvious in 401-mid-
 * session retries). Surfaces the server's error envelope on non-2xx via
 * `TokenError.body`.
 */
export async function issueTokenForConnection(args: {
  baseUrl: string;
  email: string;
  password: string;
  tenantId?: string;
  fetcher?: typeof fetch;
}): Promise<IssuedToken> {
  const fetcher = args.fetcher ?? globalThis.fetch.bind(globalThis);
  const url = `${args.baseUrl.replace(/\/+$/, '')}/api/v1/auth/token`;
  const body: Record<string, string> = {
    email: args.email,
    password: args.password,
  };
  if (args.tenantId) body.tenant_id = args.tenantId;

  let resp: Response;
  try {
    resp = await fetcher(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify(body),
    });
  } catch (e) {
    const err: TokenError = { status: 0, message: String(e) };
    throw err;
  }

  if (!resp.ok) {
    let parsed: unknown = undefined;
    try {
      parsed = await resp.json();
    } catch {
      /* empty body */
    }
    const err: TokenError = { status: resp.status, body: parsed };
    throw err;
  }
  return (await resp.json()) as IssuedToken;
}

/**
 * End-to-end "do the two steps and install the bearer" used by the
 * login screen. The provider already mounted the per-connection
 * `StarterClient`; this helper ties together step 1 (credentials POST)
 * and step 2 (install + persist).
 */
export async function loginWithCredentials(args: {
  client: StarterClient;
  secureStore: SecureTokenStore;
  connectionId: string;
  baseUrl: string;
  email: string;
  password: string;
  tenantId?: string;
}): Promise<IssuedToken> {
  const issued = await issueTokenForConnection({
    baseUrl: args.baseUrl,
    email: args.email,
    password: args.password,
    tenantId: args.tenantId,
  });
  await installBearer({
    client: args.client,
    secureStore: args.secureStore,
    connectionId: args.connectionId,
    token: issued.token,
  });
  return issued;
}
