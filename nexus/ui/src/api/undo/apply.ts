import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { UndoResponse } from "@/api/types";

// `POST /api/v1/undo` and `POST /api/v1/redo` — reverse / re-apply the caller's
// most recent change group. Undo is per-actor (the server resolves the caller's
// own redo cursor), so there is no request body. The response carries the
// `group_id` that moved, so the caller can refresh the resources it touched.
export function undo(client: StarterClient): Promise<UndoResponse> {
  return apply(client, "undo");
}

export function redo(client: StarterClient): Promise<UndoResponse> {
  return apply(client, "redo");
}

function apply(
  client: StarterClient,
  direction: "undo" | "redo",
): Promise<UndoResponse> {
  return fetchJson<UndoResponse>(client, `${client.apiPrefix}/${direction}`, {
    method: "POST",
    headers: { ...readCsrfHeader() },
  });
}
