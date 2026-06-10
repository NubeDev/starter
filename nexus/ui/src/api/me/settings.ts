import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { UserSettings } from "@/api/types";

// `GET /api/v1/me/settings` — the caller's freeform settings bag. `settings`
// is `{}` when the user has never saved any. The frontend owns the keys.
export function getUserSettings(client: StarterClient): Promise<UserSettings> {
  return fetchJson<UserSettings>(client, `${client.apiPrefix}/me/settings`);
}

// `PUT /api/v1/me/settings` — replace the caller's settings bag (full replace,
// like the tag editor). Send the whole bag; the server does not merge. Returns
// the persisted bag.
export function putUserSettings(
  client: StarterClient,
  body: UserSettings,
): Promise<UserSettings> {
  return fetchJson<UserSettings>(client, `${client.apiPrefix}/me/settings`, {
    method: "PUT",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(body),
  });
}
