import { StarterClient } from "@nube/starter-client-ts";

// Process-wide `StarterClient` — the single typed entry to `nexus-api`.
// In dev `VITE_NEXUS_BASE_URL` is empty: Vite's `server.proxy` forwards
// `/api/v1` to `nexus-api`, so the SPA issues same-origin cookie'd
// requests and avoids CORS. In prod, set `VITE_NEXUS_BASE_URL` at build
// time (e.g. `https://nexus.example.com`).
//
// This is the *only* data ingress (F2). Every binding under `api/`
// calls through this client; no raw `fetch` anywhere in `src/`.
let cached: StarterClient | null = null;

export function getNexusClient(): StarterClient {
  if (cached) return cached;
  const baseUrl = (import.meta.env.VITE_NEXUS_BASE_URL ?? "") as string;
  cached = new StarterClient({ baseUrl });
  return cached;
}
