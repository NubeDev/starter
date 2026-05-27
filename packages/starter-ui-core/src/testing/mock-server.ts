// Dependency-free fetch shim for tests. Plug it into a `StarterClient`
// via the `fetch` option:
//
//   const server = createMockServer();
//   const client = new StarterClient({ baseUrl: 'http://t', fetch: server.fetch });
//   server.setUser({ subject: '1', email: 'a@b', role: 'admin' });
//
// We deliberately don't pull in msw — its devDep tree is large and the
// surface we need to mock is tiny (three endpoints). Consumers who want
// msw can still bring it themselves; this is the turnkey option.
//
// State is mutable on the returned object so tests can flip auth state
// mid-flow without re-creating the server.
//
// Routes implemented:
// - GET  /auth/me     → 200 with current user, or 401 if unset
// - POST /auth/login  → 200 if credentials match `setLogin`, else 401
// - POST /auth/logout → 204 (clears user)
//
// Any other URL returns 404 with an RFC 7807 problem body so the
// caller sees the same error path as the real server.

import type { MeResponse } from "@nube/starter-client-ts";

export interface MockServerState {
  user: MeResponse | null;
  /** Credentials accepted by POST /auth/login. */
  validLogin: { email: string; password: string } | null;
}

export interface MockServer {
  fetch: typeof fetch;
  readonly state: MockServerState;
  setUser(user: MeResponse | null): void;
  setLogin(credentials: { email: string; password: string } | null): void;
  /** Total requests served. Useful for asserting call counts. */
  requestCount(): number;
}

export function createMockServer(initial: Partial<MockServerState> = {}): MockServer {
  const state: MockServerState = {
    user: initial.user ?? null,
    validLogin: initial.validLogin ?? null,
  };
  let count = 0;

  const fetchImpl: typeof fetch = async (input, init) => {
    count += 1;
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    const method = (init?.method ?? "GET").toUpperCase();
    const path = pathOf(url);

    if (path === "/auth/me" && method === "GET") {
      if (!state.user) return problem(401, "Unauthorized", "no session");
      return json(200, state.user);
    }

    if (path === "/auth/login" && method === "POST") {
      const body = parseJson<{ email?: string; password?: string }>(init?.body);
      if (
        state.validLogin &&
        body?.email === state.validLogin.email &&
        body?.password === state.validLogin.password
      ) {
        // Mirror the real server: login implicitly establishes the session,
        // so subsequent `me()` will succeed if the test wires `setUser`.
        return json(200, { csrf_token: "test-csrf" });
      }
      return problem(401, "Unauthorized", "bad credentials");
    }

    if (path === "/auth/logout" && method === "POST") {
      state.user = null;
      return new Response(null, { status: 204 });
    }

    return problem(404, "Not Found", `mock has no route for ${method} ${path}`);
  };

  return {
    fetch: fetchImpl,
    state,
    setUser(user) {
      state.user = user;
    },
    setLogin(credentials) {
      state.validLogin = credentials;
    },
    requestCount() {
      return count;
    },
  };
}

function pathOf(url: string): string {
  let path: string;
  try {
    path = new URL(url).pathname;
  } catch {
    const q = url.indexOf("?");
    path = q === -1 ? url : url.slice(0, q);
  }
  // StarterClient prepends `/api/v1` by default; the mock's routes are
  // declared without that prefix so they work whether the client uses
  // the default prefix or an empty one.
  if (path.startsWith("/api/v1/")) return path.slice("/api/v1".length);
  return path;
}

function parseJson<T>(body: BodyInit | null | undefined): T | undefined {
  if (typeof body !== "string") return undefined;
  try {
    return JSON.parse(body) as T;
  } catch {
    return undefined;
  }
}

function json(status: number, payload: unknown): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function problem(status: number, title: string, detail: string): Response {
  return new Response(JSON.stringify({ status, title, detail }), {
    status,
    headers: { "content-type": "application/problem+json" },
  });
}
