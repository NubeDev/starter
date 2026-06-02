// `api.ts` — thin fetch helpers shared by Main / Sidebar / NavTree.

import { EXTENSION_ID, type WarehouseQueryResponse } from "./types";

// Coalesce concurrent POSTs with identical (toolId, params) onto a
// single in-flight promise. React StrictMode double-invokes effects in
// dev, and unrelated components occasionally request the same template
// simultaneously — either way, firing the same warehouse_query twice
// wastes a round-trip and amplifies any backend slowness.
const inFlight = new Map<string, Promise<unknown>>();

// Read epoch: bumped by `invalidateReads()` after every mutation. It is
// part of the dedup key, so a read issued AFTER a write can never be
// coalesced onto a read that was already in flight BEFORE the write
// (which would observe pre-write data). Without this, provisioning a
// device and then listing devices/pages could join a stale in-flight
// list request and render the old result until a full page reload.
let readEpoch = 0;

/**
 * Invalidate the read dedup cache so subsequent reads hit the server.
 * Bumps the epoch (so the dedup key changes) AND drops every in-flight
 * read promise from the map — a read issued after a mutation must never
 * join a request that began before it (which could resolve with
 * pre-mutation data). The dropped promises still settle for whoever
 * already awaited them; they simply can't be re-shared.
 */
export function invalidateReads(): void {
  readEpoch += 1;
  inFlight.clear();
}

export function callTool<T>(toolId: string, params: unknown, opts?: { fresh?: boolean }): Promise<T> {
  const body = JSON.stringify(params ?? {});
  const key = `${toolId}::${body}::e${readEpoch}`;
  // `fresh` skips the dedup cache entirely — used for list reads where
  // serving correct, current data matters more than saving a duplicate
  // round-trip. Coalescing is only safe for genuinely concurrent reads;
  // a list re-fetched after navigation or a mutation must hit the server.
  if (!opts?.fresh) {
    const existing = inFlight.get(key);
    if (existing) return existing as Promise<T>;
  }
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
    if (inFlight.get(key) === p) inFlight.delete(key);
  });
  if (!opts?.fresh) inFlight.set(key, p);
  return p;
}

export async function fetchTemplate<R>(
  template: string,
  params: Record<string, unknown> = {},
): Promise<ReadonlyArray<R>> {
  // Always fresh: list views must reflect current data after a mutation
  // or navigation, not a coalesced/stale in-flight read.
  const res = await callTool<WarehouseQueryResponse<R>>(
    `${EXTENSION_ID}.warehouse_query`,
    { template, params },
    { fresh: true },
  );
  return res.rows;
}
