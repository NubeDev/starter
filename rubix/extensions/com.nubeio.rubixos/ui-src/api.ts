// `api.ts` — thin fetch helpers shared by Main / Sidebar / NavTree.

import { EXTENSION_ID, type WarehouseQueryResponse } from "./types";

// Coalesce concurrent POSTs with identical (toolId, params) onto a
// single in-flight promise. React StrictMode double-invokes effects in
// dev, and unrelated components occasionally request the same template
// simultaneously — either way, firing the same warehouse_query twice
// wastes a round-trip and amplifies any backend slowness.
const inFlight = new Map<string, Promise<unknown>>();

export function callTool<T>(toolId: string, params: unknown): Promise<T> {
  const body = JSON.stringify(params ?? {});
  const key = `${toolId}::${body}`;
  const existing = inFlight.get(key);
  if (existing) return existing as Promise<T>;
  const p = (async (): Promise<T> => {
    const res = await fetch(`/api/v1/tools/${toolId}`, {
      method: "POST",
      credentials: "same-origin",
      headers: { "content-type": "application/json", accept: "application/json" },
      body,
    });
    const text = await res.text();
    let parsed: unknown = undefined;
    try {
      parsed = text ? JSON.parse(text) : undefined;
    } catch {
      parsed = text;
    }
    if (!res.ok) {
      const msg =
        parsed && typeof parsed === "object" && "error" in parsed
          ? String((parsed as { error: unknown }).error)
          : `HTTP ${res.status}`;
      throw new Error(msg);
    }
    return parsed as T;
  })().finally(() => {
    inFlight.delete(key);
  });
  inFlight.set(key, p);
  return p;
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
