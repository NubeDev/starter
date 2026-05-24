// `rubix.undo.last` client method.
//
// Mutating tool call dispatched through `POST /api/v1/tools/{tool_id}`
// on rubix-agent; echoes the CSRF cookie via `readCsrfHeader()`.
// Wire shape mirrors the Rust tool definition in
// `rubix-tools/src/undo/last.rs`: input is `{ scope? }`, output is
// `{ group_id }`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";

export interface UndoLastRequest {
  /** Reserved per-resource scope filter; ignored in this release. */
  scope?: Record<string, unknown>;
}
export interface UndoLastResponse {
  /** Stable id of the undo group that was reversed. */
  group_id: string;
}

declare module "../client/client.js" {
  interface RubixClient {
    undoLast(request?: UndoLastRequest): Promise<UndoLastResponse>;
  }
}

RubixClient.prototype.undoLast = function undoLast(
  this: RubixClient,
  request: UndoLastRequest = {},
): Promise<UndoLastResponse> {
  return fetchJson<UndoLastResponse>(this.starter, `/api/v1/tools/rubix.undo.last`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
};
