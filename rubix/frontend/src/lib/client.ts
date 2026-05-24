// `getRubixClient` — process-wide singleton handle to rubix-agent.
//
// In dev, `VITE_RUBIX_BASE_URL` is intentionally empty: Vite's
// `server.proxy` (see `vite.config.ts`) forwards `/api/v1` and
// `/openapi.json` to the agent on `127.0.0.1:8088`, so the SPA can
// issue same-origin requests and avoid CORS. In prod (or when running
// against a non-default port), set `VITE_RUBIX_BASE_URL` at build
// time, e.g. `VITE_RUBIX_BASE_URL=https://rubix.example.com`.

import { StarterClient } from '@nube/starter-client-ts'
import { RubixClient } from '@nube/rubix-client-ts'

let cached: RubixClient | null = null

export function getRubixClient(): RubixClient {
  if (cached) return cached
  const baseUrl = (import.meta.env.VITE_RUBIX_BASE_URL ?? '') as string
  const starter = new StarterClient({ baseUrl })
  cached = new RubixClient(starter)
  return cached
}
