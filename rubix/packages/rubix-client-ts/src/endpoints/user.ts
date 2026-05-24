// `rubix.user.*` client methods — create, disable, list.
//
// All three dispatch through the generic tool route
// `POST /api/v1/tools/{tool_id}` exposed by rubix-agent. Wire shapes
// mirror the Rust DTOs in `rubix-spi/src/dto/user/*`.
//
// `create` and `disable` are mutating tool calls, so they echo the
// CSRF cookie via `readCsrfHeader()` from `@nube/starter-client-ts`.
// `list` is read-only but uses the same POST transport (every tool
// call is a POST), so it also threads the CSRF header for symmetry
// with the server's CSRF middleware on the `/api/v1/tools/*` mount.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface UserCreateRequest {
  email: string;
  role: string;
  password_hash?: string;
}
export interface UserCreateResponse {
  summary: Diagnostic;
  user_id: string;
  email: string;
  role: string;
  created_at_ms: number;
}

export interface UserDisableRequest {
  user_id?: string;
  email?: string;
}
export interface UserDisableResponse {
  summary: Diagnostic;
  user_id: string;
  email: string;
  role: string;
  was_already_disabled: boolean;
  disabled_at_ms: number;
}

export interface UserListRequest {}
export interface UserListItem {
  user_id: string;
  email: string;
  role: string;
  disabled_at_ms?: number;
}
export interface UserListResponse {
  summary: Diagnostic;
  count: number;
  users: UserListItem[];
}

declare module "../client/client.js" {
  interface RubixClient {
    userCreate(request: UserCreateRequest): Promise<UserCreateResponse>;
    userDisable(request: UserDisableRequest): Promise<UserDisableResponse>;
    userList(request?: UserListRequest): Promise<UserListResponse>;
  }
}

function dispatch<TReq, TRes>(client: RubixClient, toolId: string, request: TReq): Promise<TRes> {
  return fetchJson<TRes>(client.starter, `/api/v1/tools/${toolId}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request ?? {}),
  });
}

RubixClient.prototype.userCreate = function userCreate(
  this: RubixClient,
  request: UserCreateRequest,
): Promise<UserCreateResponse> {
  return dispatch(this, "rubix.user.create", request);
};

RubixClient.prototype.userDisable = function userDisable(
  this: RubixClient,
  request: UserDisableRequest,
): Promise<UserDisableResponse> {
  return dispatch(this, "rubix.user.disable", request);
};

RubixClient.prototype.userList = function userList(
  this: RubixClient,
  request: UserListRequest = {},
): Promise<UserListResponse> {
  return dispatch(this, "rubix.user.list", request);
};
