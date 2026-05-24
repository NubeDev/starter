// `rubix.clickhouse.*` client methods — rule.write, mart.create, retention.set.
//
// All three are mutating tool calls dispatched through
// `POST /api/v1/tools/{tool_id}` on rubix-agent, so they echo the
// CSRF cookie via `readCsrfHeader()`. Wire shapes mirror the Rust
// DTOs in `rubix-spi/src/dto/clickhouse/*`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface ClickhouseRuleWriteRequest {
  rule_name: string;
  ddl: string;
}
export interface ClickhouseRuleWriteResponse {
  summary: Diagnostic;
  rule_name: string;
  prior_ddl?: string;
  written_at_ms: number;
}

export interface ClickhouseMartCreateRequest {
  mart_name: string;
  ddl: string;
}
export interface ClickhouseMartCreateResponse {
  summary: Diagnostic;
  mart_name: string;
  prior_ddl?: string;
  was_already_present: boolean;
  created_at_ms: number;
}

export interface ClickhouseRetentionSetRequest {
  table_name: string;
  days: number;
}
export interface ClickhouseRetentionSetResponse {
  summary: Diagnostic;
  table_name: string;
  prior_days?: number;
  days: number;
  was_unchanged: boolean;
  set_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    ruleWrite(request: ClickhouseRuleWriteRequest): Promise<ClickhouseRuleWriteResponse>;
    martCreate(request: ClickhouseMartCreateRequest): Promise<ClickhouseMartCreateResponse>;
    retentionSet(
      request: ClickhouseRetentionSetRequest,
    ): Promise<ClickhouseRetentionSetResponse>;
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
  request: ClickhouseRuleWriteRequest,
): Promise<ClickhouseRuleWriteResponse> {
  return dispatch(this, "rubix.clickhouse.rule.write", request);
};

RubixClient.prototype.martCreate = function martCreate(
  this: RubixClient,
  request: ClickhouseMartCreateRequest,
): Promise<ClickhouseMartCreateResponse> {
  return dispatch(this, "rubix.clickhouse.mart.create", request);
};

RubixClient.prototype.retentionSet = function retentionSet(
  this: RubixClient,
  request: ClickhouseRetentionSetRequest,
): Promise<ClickhouseRetentionSetResponse> {
  return dispatch(this, "rubix.clickhouse.retention.set", request);
};
