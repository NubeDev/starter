import type {
  AddComponentRequest,
  Component,
  Edge,
  EdgeRequest,
  FlexValue,
  ReadNodesResponse,
  UpdateComponentRequest,
} from "./engine-types";
import { recordEvent } from "./instrumentation";

// All ce-rest responses are `{ data: T }` on success or `{ error: string }` on failure.
// Status codes derive from the error string (400/404/409/500/504). We unwrap once at
// the http() layer and surface plain `T` to callers; failures throw RestError.

// REST origin for the selected control engine. Defaults to same-origin
// `/api/v0` (the standalone ce-ui's Vite-proxy convention); the extension
// calls `setEngineBase("http://<ip>:<port>")` once per mounted editor so every
// request targets the chosen engine directly (Model A — direct browser→CE).
let BASE = "/api/v0";
export function setEngineBase(origin: string) {
  BASE = `${origin.replace(/\/+$/, "")}/api/v0`;
}

export class RestError extends Error {
  constructor(
    public status: number,
    message: string,
    public url: string,
    public method = "",
    public requestBody?: unknown,
    public responseBody?: unknown,
  ) {
    super(message);
  }

  // A copy-pasteable dump of the failed round-trip for debugging.
  get debug(): string {
    const fmt = (v: unknown) =>
      v === undefined ? "" : typeof v === "string" ? v : JSON.stringify(v, null, 2);
    const lines = [`${this.method} ${this.url}`, `→ ${this.status} ${this.message}`];
    if (this.requestBody !== undefined) lines.push("", "Request:", fmt(this.requestBody));
    if (this.responseBody !== undefined) lines.push("", "Response:", fmt(this.responseBody));
    return lines.join("\n");
  }
}

// Set by App.tsx after the WS schema arrives. Sent as `X-CE-Session` on every mutating
// request so the engine can attribute the resulting topology events to us — useful for
// echo suppression / origin filtering on the WS side.
let currentSessionId: string | null = null;
export function setRestSessionId(id: string | null) {
  currentSessionId = id;
}

async function http<T>(method: string, path: string, body?: unknown): Promise<T> {
  const url = `${BASE}${path}`;
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (currentSessionId && method !== "GET") headers["X-CE-Session"] = currentSessionId;
  const res = await fetch(url, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  let payload: { data?: T; error?: string } = {};
  try {
    payload = (await res.json()) as typeof payload;
  } catch {
    // Empty body — e.g. some 5xx paths. Fall through; status determines outcome.
  }
  // Log mutations to the events panel. GETs would dominate the log so we
  // skip them — the panel is for tracking writes and the topology / value
  // pushes they trigger.
  if (method !== "GET") {
    const status = res.ok && !payload.error ? res.status : `ERR ${res.status}`;
    recordEvent("rest", `${method} ${path} → ${status}`);
  }
  if (!res.ok || payload.error) {
    throw new RestError(res.status, payload.error ?? res.statusText, url, method, body, payload);
  }
  return payload.data as T;
}

// Components ----------------------------------------------------------------

export function getRootNodes(opts?: {
  depth?: number;
  nested?: boolean;
  withEdges?: boolean;
}): Promise<ReadNodesResponse> {
  const q = new URLSearchParams();
  if (opts?.depth != null) q.set("depth", String(opts.depth));
  if (opts?.nested != null) q.set("nested", String(opts.nested));
  if (opts?.withEdges != null) q.set("withEdges", String(opts.withEdges));
  const qs = q.toString();
  return http<ReadNodesResponse>("GET", `/nodes${qs ? "?" + qs : ""}`);
}

export function getNodeByUid(
  uid: number,
  opts?: { depth?: number; nested?: boolean; withEdges?: boolean },
) {
  const q = new URLSearchParams();
  if (opts?.depth != null) q.set("depth", String(opts.depth));
  if (opts?.nested != null) q.set("nested", String(opts.nested));
  if (opts?.withEdges != null) q.set("withEdges", String(opts.withEdges));
  const qs = q.toString();
  return http<ReadNodesResponse>("GET", `/nodes/uid/${uid}${qs ? "?" + qs : ""}`);
}

export function addNode(req: AddComponentRequest) {
  return http<Component>("POST", `/nodes`, req);
}

export function updateNode(uid: number, req: UpdateComponentRequest) {
  return http<Component>("PATCH", `/nodes/uid/${uid}`, req);
}

// Override endpoints. Setting an override pins a property's value so cycle
// execution doesn't change it; clearing returns the property to engine control.
// `duration` is the override lifetime in seconds (0 = permanent).
export interface PropertyOverride {
  property: string;
  value: FlexValue;
  duration?: number;
}
export interface OverridesRequest {
  setOverrides?: PropertyOverride[];
  clearOverrides?: string[];
}
export function patchOverrides(uid: number, req: OverridesRequest) {
  return http<Component>("PATCH", `/overrides/nodes/uid/${uid}`, req);
}

// Action dispatch — `POST /call/nodes/uid/{uid}`. Invoke one named action with
// an optional params map; the engine returns the action's `returns` values. The
// action's signature (which params/returns it has) is type-level and comes from
// `/schema`; this call just dispatches it on one component instance.
export function callAction(
  uid: number,
  action: string,
  params?: Record<string, FlexValue>,
): Promise<{ returns: Record<string, FlexValue> }> {
  return http<{ returns: Record<string, FlexValue> }>("POST", `/call/nodes/uid/${uid}`, {
    action,
    params,
  });
}

export function removeNode(uid: number) {
  return http<unknown>("DELETE", `/nodes/uid/${uid}`);
}

// Edges ---------------------------------------------------------------------

export function getEdges(component?: string | number) {
  const qs = component != null ? `?component=${encodeURIComponent(String(component))}` : "";
  return http<Edge[]>("GET", `/edges${qs}`);
}

export function addEdge(req: EdgeRequest) {
  return http<{ uid: number } & Edge>("POST", `/edge`, req);
}

// Result shape shared by the bulk write endpoints. componentUids/edgeUids are
// the survivors (failed inputs are absent); `partialSuccess` + `errors` carry
// per-item failures (HTTP 207). (Client no longer creates from specs — paste
// uses /copy/nodes, undo-delete uses /restore — but bulkUpdate still returns
// this shape.)
export interface BulkAddResponse {
  components?: Array<{ uid: number; name: string; path: string }>;
  edges?: Array<{ uid: number }>;
  result?: {
    componentUids?: number[];
    edgeUids?: number[];
    partialSuccess?: boolean;
    errors?: {
      components?: Array<{ inputIndex: number; message: string }>;
      edges?: Array<{ inputIndex: number; message: string }>;
    };
    componentErrors?: Array<{ inputIndex: number; message: string }>;
    edgeErrors?: Array<{ inputIndex: number; message: string }>;
  };
}

// PATCH /bulknodes — atomic multi-component update. Shares its body shape
// with PATCH /nodes/uid/{uid} per entry. Used to batch group drag moves so
// 8-component multi-select drag at 10 Hz doesn't fire 80 PATCHes/sec.
export interface BulkUpdateEntry {
  uid: number;
  name?: string;
  parentUid?: number;
  position?: { x?: number; y?: number };
  properties?: Record<string, { value: FlexValue }>;
}

export function bulkUpdate(updates: BulkUpdateEntry[]): Promise<BulkAddResponse> {
  return http<BulkAddResponse>("PATCH", `/bulknodes`, { updates });
}

// DELETE /bulknodes — atomic multi-component / multi-edge removal (soft-delete;
// restorable via POST /restore).
export interface UidLists {
  componentUids?: number[];
  edgeUids?: number[];
}
export function bulkDelete(req: UidLists): Promise<unknown> {
  return http<unknown>("DELETE", `/bulknodes`, req);
}

// POST /copy/nodes — clone components (and their internal edges) by UID under a
// destination parent. Server-side clone: full fidelity, tiny request. Returns
// the created components (full shape, incl. positions copied from the source).
// This is how paste works — no client-side spec reconstruction.
export function copyNodes(req: {
  componentUids: number[];
  destParentUid: number;
  includeInternalEdges?: boolean;
}): Promise<ReadNodesResponse> {
  return http<ReadNodesResponse>("POST", `/copy/nodes`, {
    includeInternalEdges: true,
    ...req,
  });
}

// POST /restore — restore soft-deleted components/edges by their ORIGINAL uids
// (full state). This is undo-of-delete: strictly better than recreating from a
// snapshot (keeps uids, full values, and the deleted subtree).
export function restoreItems(req: UidLists): Promise<unknown> {
  return http<unknown>("POST", `/restore`, req);
}

// Returns a human-readable summary if a bulk response is a partial success
// (HTTP 207 — some items failed). `res.ok` is true for 207 so http() returns
// normally; without this check the failures are invisible. Callers surface it
// without throwing, so the items that DID succeed still apply.
export function bulkPartialError(res: BulkAddResponse): string | null {
  if (!res.result?.partialSuccess) return null;
  const errs = [
    ...(res.result.errors?.components ?? res.result.componentErrors ?? []),
    ...(res.result.errors?.edges ?? res.result.edgeErrors ?? []),
  ];
  if (errs.length === 0) return "bulk op partially failed";
  const first = errs[0];
  return `bulk op: ${errs.length} item(s) failed — ${first.message}`;
}

export function removeEdge(uid: number) {
  return http<unknown>("DELETE", `/edge/uid/${uid}`);
}

// PATCH /edge/uid/{uid}. `reEvaluate: true` re-fires the source value through
// the edge once — useful when the downstream component's last-computed value
// stuck and you want to nudge a recompute without changing the source.
export interface UpdateEdgeRequest {
  loopBack?: boolean;
  hidden?: boolean;
  reEvaluate?: boolean;
}

export function updateEdge(uid: number, req: UpdateEdgeRequest) {
  return http<Edge>("PATCH", `/edge/uid/${uid}`, req);
}

// System / schema -----------------------------------------------------------

export interface ExtensionInfo {
  name: string;
  version?: string;
  components?: string[];
}

export function getExtensions() {
  return http<ExtensionInfo[]>("GET", `/extensions`);
}
