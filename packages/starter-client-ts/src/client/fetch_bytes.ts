// Shared raw-bytes fetch helper. Returns the response body as an
// `ArrayBuffer` for endpoints that download binary blobs.

import type { StarterClient } from "./client.js";
import { StarterError } from "../error/starter-error.js";

export async function fetchBytes(
  client: StarterClient,
  path: string,
  init: RequestInit = {},
): Promise<ArrayBuffer> {
  const headers: Record<string, string> = { ...client.headers, ...(init.headers as Record<string, string> | undefined) };
  const res = await client.fetch(`${client.baseUrl}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  return await res.arrayBuffer();
}
