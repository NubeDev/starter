// `api.ts` — thin fetch helpers shared by Main / Sidebar / NavTree.

import { EXTENSION_ID, type WarehouseQueryResponse } from "./types";

export async function callTool<T>(toolId: string, params: unknown): Promise<T> {
  const res = await fetch(`/api/v1/tools/${toolId}`, {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify(params ?? {}),
  });
  const text = await res.text();
  let body: unknown = undefined;
  try {
    body = text ? JSON.parse(text) : undefined;
  } catch {
    body = text;
  }
  if (!res.ok) {
    const msg =
      body && typeof body === "object" && "error" in body
        ? String((body as { error: unknown }).error)
        : `HTTP ${res.status}`;
    throw new Error(msg);
  }
  return body as T;
}

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
