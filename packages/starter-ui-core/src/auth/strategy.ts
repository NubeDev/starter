// Pluggable auth strategy. Three impls ship in this package; consumers
// can write their own (e.g. external IdP) as long as it matches this
// shape.
//
// The provider owns the StarterClient; strategies receive it on each
// call rather than holding a reference, so a single client instance
// can swap strategies without re-wiring.

import type { StarterClient, MeResponse, LoginRequest } from "@nube/starter-client-ts";

export interface AuthStrategy {
  /** Strategy name for diagnostics + DevTools. */
  readonly kind: "session" | "token" | "external";

  /** Whoami probe. Returns `null` if unauthenticated, the user on success. */
  load(client: StarterClient): Promise<MeResponse | null>;

  /** Log in. Implementations differ wildly; `session` takes credentials,
   * `token` takes a bearer string, `external` redirects. */
  login(client: StarterClient, input: LoginInput): Promise<MeResponse>;

  /** Log out. Idempotent — already-logged-out is not an error. */
  logout(client: StarterClient): Promise<void>;
}

export type LoginInput =
  | { kind: "credentials"; email: string; password: string }
  | { kind: "token"; token: string }
  | { kind: "external" };

/** Cookie-session strategy. `login` POSTs `/auth/login`, `logout` POSTs
 * `/auth/logout` with the CSRF header echoed from the cookie. */
export const sessionStrategy: AuthStrategy = {
  kind: "session",
  async load(client) {
    try {
      return await client.me();
    } catch {
      return null;
    }
  },
  async login(client, input) {
    if (input.kind !== "credentials") {
      throw new Error(`sessionStrategy.login requires { kind: 'credentials' }, got '${input.kind}'`);
    }
    const req: LoginRequest = { email: input.email, password: input.password };
    await client.login(req);
    const me = await client.me();
    if (!me) throw new Error("login succeeded but /auth/me returned null");
    return me;
  },
  async logout(client) {
    await client.logout();
  },
};

/** Bearer-token strategy. The token is held in memory by the provider
 * and attached as `Authorization: Bearer …` via `client.headers`. */
export function tokenStrategy(opts: { onTokenChange?: (token: string | null) => void } = {}): AuthStrategy {
  return {
    kind: "token",
    async load(client) {
      try {
        return await client.me();
      } catch {
        return null;
      }
    },
    async login(client, input) {
      if (input.kind !== "token") {
        throw new Error(`tokenStrategy.login requires { kind: 'token' }, got '${input.kind}'`);
      }
      client.headers["Authorization"] = `Bearer ${input.token}`;
      opts.onTokenChange?.(input.token);
      const me = await client.me();
      if (!me) throw new Error("token rejected — /auth/me returned null");
      return me;
    },
    async logout(client) {
      delete client.headers["Authorization"];
      opts.onTokenChange?.(null);
    },
  };
}

/** External IdP strategy. `login` triggers the redirect; `load` checks
 * the session after the IdP round-trip completes. Both delegate to the
 * caller-supplied URLs. */
export function externalStrategy(opts: { loginUrl: string; logoutUrl: string }): AuthStrategy {
  return {
    kind: "external",
    async load(client) {
      try {
        return await client.me();
      } catch {
        return null;
      }
    },
    async login() {
      if (typeof window === "undefined") {
        throw new Error("externalStrategy.login requires a browser environment");
      }
      window.location.assign(opts.loginUrl);
      return new Promise<never>(() => {});
    },
    async logout() {
      if (typeof window === "undefined") return;
      window.location.assign(opts.logoutUrl);
    },
  };
}
