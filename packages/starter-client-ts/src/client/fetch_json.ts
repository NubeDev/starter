// Shared JSON fetch helper. Builds the full URL from the client's
// base, forces `credentials: "include"` (cookie auth), throws a typed
// `StarterError` on non-2xx, and returns the parsed body as `T`.
//
// A 2xx response whose `content-type` is not JSON also throws a
// typed `StarterError` via `StarterError.invalidResponse`. This
// catches dev-server SPA-fallback misroutes (vite serves `index.html`
// for unknown paths, which would otherwise blow up `res.json()` with
// an opaque `SyntaxError` and silently bypass auth guards downstream).
//
// Endpoint modules use this so each method collapses to ~5 lines.

import type { StarterClient } from "./client.js";
import { StarterError } from "../error/starter-error.js";

function isJsonContentType(value: string | null): boolean {
  if (!value) return false;
  const semi = value.indexOf(";");
  const main = (semi === -1 ? value : value.slice(0, semi)).trim().toLowerCase();
  return main === "application/json" || main === "application/problem+json" || main.endsWith("+json");
}

export async function fetchJson<T>(
  client: StarterClient,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers: Record<string, string> = { ...client.headers, ...(init.headers as Record<string, string> | undefined) };
  const url = `${client.baseUrl}${path}`;
  const res = await client.fetch(url, {
    ...init,
    credentials: "include",
    headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  if (!isJsonContentType(res.headers.get("content-type"))) {
    throw StarterError.invalidResponse(url, res.headers.get("content-type"));
  }
  return (await res.json()) as T;
}
