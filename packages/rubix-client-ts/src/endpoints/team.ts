// `rubix.team.*` client methods — create, assign.
//
// Both dispatch through `POST /api/v1/tools/{tool_id}` on rubix-agent.
// Wire shapes mirror the Rust DTOs in `rubix-spi/src/dto/team/*`.
// Both are mutating, so they echo the CSRF cookie via
// `readCsrfHeader()`.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface TeamCreateRequest {
  name: string;
  description?: string;
}
export interface TeamCreateResponse {
  summary: Diagnostic;
  team_id: string;
  name: string;
  created_at_ms: number;
}

export interface TeamAssignRequest {
  team_id: string;
  user_id: string;
}
export interface TeamAssignResponse {
  summary: Diagnostic;
  team_id: string;
  user_id: string;
  already_member: boolean;
  assigned_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    teamCreate(request: TeamCreateRequest): Promise<TeamCreateResponse>;
    teamAssign(request: TeamAssignRequest): Promise<TeamAssignResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}

RubixClient.prototype.teamCreate = function teamCreate(
  this: RubixClient,
  request: TeamCreateRequest,
): Promise<TeamCreateResponse> {
  return dispatch(this, "rubix.team.create", request);
};

RubixClient.prototype.teamAssign = function teamAssign(
  this: RubixClient,
  request: TeamAssignRequest,
): Promise<TeamAssignResponse> {
  return dispatch(this, "rubix.team.assign", request);
};
