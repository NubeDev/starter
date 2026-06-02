// `api.ts` — transport for tool calls + warehouse-template reads.
//
// SCAFFOLD: a plain POST to `/api/v1/tools/<id>`. No dedup, no caching,
// no invalidation — add those (see com.nubeio.rubixos/ui-src/api.ts)
// once the panels need them.

import { EXTENSION_ID, type WarehouseQueryResponse } from "./types";

export async function callTool<T>(toolId: string, params: unknown): Promise<T> {
  const res = await fetch(`/api/v1/tools/${toolId}`, {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(params ?? {}),
  });
  // TODO: error handling.
  return (await res.json()) as T;
}

/** Run one of this extension's warehouse_templates via the proxy tool. */
export async function fetchTemplate<R>(
  template: string,
  params: Record<string, unknown> = {},
): Promise<ReadonlyArray<R>> {
  const res = await callTool<WarehouseQueryResponse<R>>(
    `${EXTENSION_ID}.warehouse_query`,
    { template, params },
  );
  return res.rows;
}
