// `useAudit` — read hook for the `/v1/audit` paged user-audit
// projection on starter-server.
//
// Audit is intentionally not part of the rubix-client-ts endpoint
// barrel (SCOPE OQ-3) — the route lives on starter-server, not
// rubix-agent. Until a typed audit endpoint lands on
// `@nube/starter-client-ts`, this hook calls `fetchJson` directly
// against the wrapped starter client. The hook shape matches the
// other read families so swapping to a typed `client.starter.audit*`
// is a one-line change at the call site.

import { useQuery, type UseQueryOptions, type UseQueryResult } from "@tanstack/react-query";

import { fetchJson, type StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const AUDIT_KEY = ["rubix", "audit"] as const;

/** Mirror of `starter_audit::ChangeFilter`. Kept loose until the typed surface lands. */
export interface AuditFilter {
  resource_kind?: string;
  resource_id?: string;
  actor_id?: string;
  group_id?: string;
  trace_id?: string;
  since_ms?: number;
  until_ms?: number;
  limit?: number;
  cursor?: string;
}

/** Mirror of `starter_audit::ChangePage`. Loosened to `unknown[]` until typed. */
export interface AuditPage {
  changes: unknown[];
  next_cursor?: string;
}

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;

function buildQuery(filter: AuditFilter): string {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(filter)) {
    if (v !== undefined && v !== null) params.set(k, String(v));
  }
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

/**
 * Paged user-audit list. Query key:
 * `['rubix','audit','list', filter]` — the filter object is part of
 * the key so distinct filters cache independently.
 */
export function useAudit(
  filter: AuditFilter = {},
  options?: ReadOptions<AuditPage>,
): UseQueryResult<AuditPage, StarterError> {
  const client = useRubixClient();
  return useQuery<AuditPage, StarterError>({
    queryKey: [...AUDIT_KEY, "list", filter],
    queryFn: () => fetchJson<AuditPage>(client.starter, `/v1/audit${buildQuery(filter)}`),
    ...options,
  });
}
