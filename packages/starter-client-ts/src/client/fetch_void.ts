// Shared no-content fetch helper. Same shape as `fetchJson` but
// discards the response body — for `204 No Content` mutations and
// fire-and-forget POST/DELETE endpoints.

import type { StarterClient } from "./client.js";
import { StarterError } from "../error/starter-error.js";

export async function fetchVoid(
  client: StarterClient,
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  const headers: Record<string, string> = { ...client.headers, ...(init.headers as Record<string, string> | undefined) };
  const res = await client.fetch(`${client.baseUrl}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  return res;
}
