// `rubix.flow_ops.*` client methods — deploy, lint, list, duplicate.
//
// All four dispatch through `POST /api/v1/tools/{tool_id}` on
// rubix-agent. `lint` and `list` are read-only but use the same
// POST transport; every mutating method (and for symmetry the
// read-only ones too) thread the CSRF cookie via `readCsrfHeader()`
// because the `/api/v1/tools/*` mount is CSRF-gated.
// Wire shapes mirror the Rust DTOs in `rubix-spi/src/dto/flow_ops/*`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface FlowDeployRequest {
  flow_id: string;
  body_yaml: string;
}
export interface FlowDeployResponse {
  summary: Diagnostic;
  flow_id: string;
  revision_id: string;
  prior_revision_id?: string;
  deployed_at_ms: number;
}

export interface FlowLintRequest {
  body_yaml: string;
}
export interface LintDiagnostic {
  message: string;
  line?: number;
  column?: number;
}
export interface FlowLintResponse {
  summary: Diagnostic;
  errors: LintDiagnostic[];
}

export interface FlowListRequest {}
export interface FlowListItem {
  flow_id: string;
  revision_id: string;
}
export interface FlowListResponse {
  summary: Diagnostic;
  count: number;
  flows: FlowListItem[];
}

export interface FlowDuplicateRequest {
  source_flow_id: string;
  target_flow_id: string;
}
export interface FlowDuplicateResponse {
  summary: Diagnostic;
  source_flow_id: string;
  target_flow_id: string;
  revision_id: string;
  created_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    flowDeploy(request: FlowDeployRequest): Promise<FlowDeployResponse>;
    flowLint(request: FlowLintRequest): Promise<FlowLintResponse>;
    flowList(request?: FlowListRequest): Promise<FlowListResponse>;
    flowDuplicate(request: FlowDuplicateRequest): Promise<FlowDuplicateResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request ?? {}),
  });
}

RubixClient.prototype.flowDeploy = function flowDeploy(
  this: RubixClient,
  request: FlowDeployRequest,
): Promise<FlowDeployResponse> {
  return dispatch(this, "rubix.flow_ops.deploy", request);
};

RubixClient.prototype.flowLint = function flowLint(
  this: RubixClient,
  request: FlowLintRequest,
): Promise<FlowLintResponse> {
  return dispatch(this, "rubix.flow_ops.lint", request);
};

RubixClient.prototype.flowList = function flowList(
  this: RubixClient,
  request: FlowListRequest = {},
): Promise<FlowListResponse> {
  return dispatch(this, "rubix.flow_ops.list", request);
};

RubixClient.prototype.flowDuplicate = function flowDuplicate(
  this: RubixClient,
  request: FlowDuplicateRequest,
): Promise<FlowDuplicateResponse> {
  return dispatch(this, "rubix.flow_ops.duplicate", request);
};
