// `rubix.system.*` client methods.
//
// All three verbs dispatch through the generic tool route
// `POST /api/v1/tools/{tool_id}` exposed by rubix-agent (see
// `rubix/crates/rubix-agent/src/routes/tools.rs`). The request body
// is the tool DTO; the response body carries a `summary` Diagnostic
// plus the structured probe data — wire shapes match the Rust DTOs
// in `rubix-spi/src/dto/system/*`.
//
// Endpoint methods hang off `RubixClient` via declaration-merging
// and delegate transport to the wrapped `StarterClient` through
// `fetchJson` (cookie auth + JSON content-type + typed-error throw
// are all handled there).

import { fetchJson } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";

/** Keyed i18n outcome carried by every rubix tool response. */
export interface Diagnostic {
  code: string;
  params?: Record<string, unknown>;
}

export interface DiskUsageRequest {
  mount?: string;
}
export interface DiskUsageResponse {
  summary: Diagnostic;
  mount: string;
  total_bytes: number;
  free_bytes: number;
  percent_used: number;
  probed_at_ms: number;
}

export interface DbHealthRequest {
  dsn?: string;
}
export interface DbHealthResponse {
  summary: Diagnostic;
  dsn: string;
  reachable: boolean;
  used_bytes: number;
  probed_at_ms: number;
}

export interface FlowErrorsRequest {
  window_secs?: number;
}
export interface FlowErrorSample {
  flow_id: string;
  message: string;
  at_ms: number;
}
export interface FlowErrorsResponse {
  summary: Diagnostic;
  window_secs: number;
  error_count: number;
  samples: FlowErrorSample[];
  probed_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    disk(request?: DiskUsageRequest): Promise<DiskUsageResponse>;
    db(request?: DbHealthRequest): Promise<DbHealthResponse>;
    flowErrors(request?: FlowErrorsRequest): Promise<FlowErrorsResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(request ?? {}),
  });
}

RubixClient.prototype.disk = function disk(
  this: RubixClient,
  request: DiskUsageRequest = {},
): Promise<DiskUsageResponse> {
  return dispatch(this, "rubix.system.disk", request);
};

RubixClient.prototype.db = function db(
  this: RubixClient,
  request: DbHealthRequest = {},
): Promise<DbHealthResponse> {
  return dispatch(this, "rubix.system.db", request);
};

RubixClient.prototype.flowErrors = function flowErrors(
  this: RubixClient,
  request: FlowErrorsRequest = {},
): Promise<FlowErrorsResponse> {
  return dispatch(this, "rubix.system.flow_errors", request);
};
