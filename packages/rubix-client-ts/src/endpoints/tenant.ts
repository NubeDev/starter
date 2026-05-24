// `rubix.tenant.list` client method.
//
// Dispatches through `POST /api/v1/tools/rubix.tenant.list` on
// rubix-agent. Wire shape mirrors the Rust DTO in
// `rubix-spi/src/dto/tenant/list.rs`. Read-only, but uses the same
// tool POST transport, so it threads the CSRF cookie via
// `readCsrfHeader()` to satisfy the server's CSRF middleware on the
// `/api/v1/tools/*` mount.

import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

export interface TenantListRequest {}
export interface TenantListItem {
  tenant_id: string;
  name: string;
  locale: string;
}
export interface TenantListResponse {
  summary: Diagnostic;
  count: number;
  tenants: TenantListItem[];
}

declare module "../client/client.js" {
  interface RubixClient {
    tenantList(request?: TenantListRequest): Promise<TenantListResponse>;
  }
}

RubixClient.prototype.tenantList = function tenantList(
  this: RubixClient,
  request: TenantListRequest = {},
): Promise<TenantListResponse> {
  return fetchJson<TenantListResponse>(this.starter, `/api/v1/tools/rubix.tenant.list`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
};
