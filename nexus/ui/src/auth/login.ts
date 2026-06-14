import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

export interface LoginRequest {
  email: string;
  password: string;
}

// `starter-auth-users` mounts the cookie-session auth at the server *root*
// (`/auth/*`), not under the `/api/v1` product surface — so these bindings
// use absolute paths, not `client.apiPrefix`. (The starter client's own
// `login()`/`me()` assume `/api/v1/auth/*`, which 404s against nexus-api;
// that's why Nexus has its own auth layer.) Login returns the CSRF token
// and sets the session cookie; subsequent mutations echo the token via
// `readCsrfHeader`.
export function login(
  client: StarterClient,
  request: LoginRequest,
): Promise<{ csrf_token: string }> {
  return fetchJson<{ csrf_token: string }>(client, "/auth/login", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(request),
  });
}

export async function logout(client: StarterClient): Promise<void> {
  await fetchVoid(client, "/auth/logout", {
    method: "POST",
    headers: readCsrfHeader(),
  });
}
