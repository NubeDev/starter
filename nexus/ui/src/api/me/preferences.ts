import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";
import type {
  PreferencesPatch,
  ResolvedPreferences,
} from "@nube/starter-ui-core/preferences";

// `GET /api/v1/me/preferences` — the caller's resolved preferences
// (user → org → system default), resolved server-side for the
// principal's own tenant. Unlike the starter platform's stock fetcher,
// nexus does NOT take a spoofable `?org=` selector: isolation is
// route-pinned from the authenticated principal (WS-11). Cookie session
// auth is handled by `fetchJson` (credentials: include).
export function getMyPreferences(
  client: StarterClient,
): Promise<ResolvedPreferences> {
  return fetchJson<ResolvedPreferences>(
    client,
    `${client.apiPrefix}/me/preferences`,
  );
}

// `PATCH /api/v1/me/preferences` — merge a partial update into the
// caller's own preferences and return the freshly resolved view. A
// `null` field value means "revert to inherit" (org → default) per the
// Rust route layer.
export function patchMyPreferences(
  client: StarterClient,
  patch: PreferencesPatch,
): Promise<ResolvedPreferences> {
  return fetchJson<ResolvedPreferences>(
    client,
    `${client.apiPrefix}/me/preferences`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(patch),
    },
  );
}
