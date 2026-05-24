// Shared JSON fetch helper. Builds the full URL from the client's
// base, forces `credentials: "include"` (cookie auth), throws a typed
// `StarterError` on non-2xx, and returns the parsed body as `T`.
//
// Endpoint modules use this so each method collapses to ~5 lines.

import type { StarterClient } from "./client.js";
import { StarterError } from "../error/starter-error.js";

export async function fetchJson<T>(
  client: StarterClient,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers: Record<string, string> = { ...client.headers, ...(init.headers as Record<string, string> | undefined) };
  const res = await client.fetch(`${client.baseUrl}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  return (await res.json()) as T;
}
