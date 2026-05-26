// `rubix.warehouse.*` client methods — rule.write, mart.create, retention.set.
//
// All three are mutating tool calls dispatched through
// `POST /api/v1/tools/{tool_id}` on rubix-agent, so they echo the
// CSRF cookie via `readCsrfHeader()`. Wire shapes mirror the Rust
// DTOs in `rubix-spi/src/dto/warehouse/*`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface WarehouseRuleWriteRequest {
  rule_name: string;
  ddl: string;
}
export interface WarehouseRuleWriteResponse {
  summary: Diagnostic;
  rule_name: string;
  prior_ddl?: string;
  written_at_ms: number;
}

export interface WarehouseMartCreateRequest {
  mart_name: string;
  ddl: string;
}
export interface WarehouseMartCreateResponse {
  summary: Diagnostic;
  mart_name: string;
  prior_ddl?: string;
  was_already_present: boolean;
  created_at_ms: number;
}

export interface WarehouseRetentionSetRequest {
  table_name: string;
  days: number;
}
export interface WarehouseRetentionSetResponse {
  summary: Diagnostic;
  table_name: string;
  prior_days?: number;
  days: number;
  was_unchanged: boolean;
  set_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    ruleWrite(request: WarehouseRuleWriteRequest): Promise<WarehouseRuleWriteResponse>;
    martCreate(request: WarehouseMartCreateRequest): Promise<WarehouseMartCreateResponse>;
    retentionSet(
      request: WarehouseRetentionSetRequest,
    ): Promise<WarehouseRetentionSetResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}

RubixClient.prototype.ruleWrite = function ruleWrite(
  this: RubixClient,
  request: WarehouseRuleWriteRequest,
): Promise<WarehouseRuleWriteResponse> {
  return dispatch(this, "rubix.warehouse.rule.write", request);
};

RubixClient.prototype.martCreate = function martCreate(
  this: RubixClient,
  request: WarehouseMartCreateRequest,
): Promise<WarehouseMartCreateResponse> {
  return dispatch(this, "rubix.warehouse.mart.create", request);
};

RubixClient.prototype.retentionSet = function retentionSet(
  this: RubixClient,
  request: WarehouseRetentionSetRequest,
): Promise<WarehouseRetentionSetResponse> {
  return dispatch(this, "rubix.warehouse.retention.set", request);
};
