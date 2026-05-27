// `rubix.dashboard.*` client methods — create, update, get, list,
// duplicate, delete, page_set.
//
// All seven dispatch through `POST /api/v1/tools/{tool_id}` on
// rubix-agent. Read verbs (`get`, `list`) use the same POST
// transport for symmetry. Every method threads the CSRF cookie via
// `readCsrfHeader()` because the `/api/v1/tools/*` mount is
// CSRF-gated. Wire shapes mirror the Rust DTOs in
// `rubix-spi/src/dto/dashboard/*`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface DashboardPageSummary {
  page_id: string;
  revision_id: string;
  title: string;
  tags: string[];
  owner_principal: string;
  updated_at: string;
}

export interface DashboardPage extends DashboardPageSummary {
  body_json: unknown;
}

export interface DashboardGetRequest {
  tenant_id: string;
  page_id: string;
}
export interface DashboardGetResponse {
  summary: Diagnostic;
  page_id: string;
  revision_id?: string;
  tenant_id?: string;
  owner_principal?: string;
  title?: string;
  tags?: string[];
  body_json?: unknown;
  created_by?: string;
  created_at?: string;
}

export interface DashboardListRequest {
  tenant_id: string;
  /** Optional substring filter against `page_id`. */
  filter?: string;
}
export interface DashboardListResponse {
  summary: Diagnostic;
  count: number;
  items: DashboardPageSummary[];
}

export interface DashboardCreateRequest {
  page_id: string;
  title: string;
  tags?: string[];
  body_json: unknown;
}
export interface DashboardCreateResponse {
  summary: Diagnostic;
  page_id: string;
  revision_id: string;
  created_at_ms: number;
}

export interface DashboardUpdateRequest {
  tenant_id: string;
  page_id: string;
  expected_revision_id: string;
  title?: string;
  tags?: string[];
  body_json: unknown;
  created_by: string;
}
export interface DashboardUpdateResponse {
  summary: Diagnostic;
  page_id: string;
  revision_id: string;
  prior_revision_id: string;
  updated_at_ms: number;
}

export interface DashboardDeleteRequest {
  page_id: string;
}
export interface DashboardDeleteResponse {
  summary: Diagnostic;
  page_id: string;
  deleted_at_ms: number;
}

export interface DashboardDuplicateRequest {
  source_page_id: string;
  target_page_id: string;
}
export interface DashboardDuplicateResponse {
  summary: Diagnostic;
  source_page_id: string;
  target_page_id: string;
  revision_id: string;
  created_at_ms: number;
}

export interface DashboardPageSetRequest {
  node_id: string;
  slot: string;
  value: unknown;
}
export interface DashboardPageSetResponse {
  summary: Diagnostic;
  node_id: string;
  slot: string;
  applied_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    dashboardGet(request: DashboardGetRequest): Promise<DashboardGetResponse>;
    dashboardList(request: DashboardListRequest): Promise<DashboardListResponse>;
    dashboardCreate(request: DashboardCreateRequest): Promise<DashboardCreateResponse>;
    dashboardUpdate(request: DashboardUpdateRequest): Promise<DashboardUpdateResponse>;
    dashboardDelete(request: DashboardDeleteRequest): Promise<DashboardDeleteResponse>;
    dashboardDuplicate(
      request: DashboardDuplicateRequest,
    ): Promise<DashboardDuplicateResponse>;
    dashboardPageSet(request: DashboardPageSetRequest): Promise<DashboardPageSetResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request ?? {}),
  });
}

RubixClient.prototype.dashboardGet = function dashboardGet(
  this: RubixClient,
  request: DashboardGetRequest,
): Promise<DashboardGetResponse> {
  return dispatch(this, "rubix.dashboard.get", request);
};

RubixClient.prototype.dashboardList = function dashboardList(
  this: RubixClient,
  request: DashboardListRequest,
): Promise<DashboardListResponse> {
  return dispatch(this, "rubix.dashboard.list", request);
};

RubixClient.prototype.dashboardCreate = function dashboardCreate(
  this: RubixClient,
  request: DashboardCreateRequest,
): Promise<DashboardCreateResponse> {
  return dispatch(this, "rubix.dashboard.create", request);
};

RubixClient.prototype.dashboardUpdate = function dashboardUpdate(
  this: RubixClient,
  request: DashboardUpdateRequest,
): Promise<DashboardUpdateResponse> {
  return dispatch(this, "rubix.dashboard.update", request);
};

RubixClient.prototype.dashboardDelete = function dashboardDelete(
  this: RubixClient,
  request: DashboardDeleteRequest,
): Promise<DashboardDeleteResponse> {
  return dispatch(this, "rubix.dashboard.delete", request);
};

RubixClient.prototype.dashboardDuplicate = function dashboardDuplicate(
  this: RubixClient,
  request: DashboardDuplicateRequest,
): Promise<DashboardDuplicateResponse> {
  return dispatch(this, "rubix.dashboard.duplicate", request);
};

RubixClient.prototype.dashboardPageSet = function dashboardPageSet(
  this: RubixClient,
  request: DashboardPageSetRequest,
): Promise<DashboardPageSetResponse> {
  return dispatch(this, "rubix.dashboard.page_set", request);
};
