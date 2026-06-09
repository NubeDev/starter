// Grafana-style "Share dashboard" surface.
//
// Reuses the generic permissions drawer from `@nube/starter-ui-authz` (the same
// one Rubix uses for its pages), pointed at the `nexus.dashboard` resource kind.
// The drawer renders the share-scope radios (Private / Anyone in tenant /
// Specific) and the per-grant tier + remove rows; all mutations go straight to
// the `/v1/authz/*` endpoints on the shared client.
//
// We fetch the one dashboard's instance (its effective ACL + label) via
// `listResourceInstances` and hand it to the drawer. The authz routes are not in
// nexus's OpenAPI, so these calls use the StarterClient authz methods directly,
// not the codegen'd nexus client.

import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import type { ResourceInstance } from "@nube/starter-client-ts";
import { PageDetailDrawer } from "@nube/starter-ui-authz";

import { usePrincipal } from "@/auth/usePrincipal";

const DASHBOARD_KIND = "nexus.dashboard";

export interface ShareDashboardDialogProps {
  /** Dashboard immutable id (grants key on the id, never the slug). */
  dashboardId: string;
  /** Open state controlled by the toolbar's Share button. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ShareDashboardDialog({
  dashboardId,
  open,
  onOpenChange,
}: ShareDashboardDialogProps) {
  const client = useStarterClient();
  const principal = usePrincipal();
  const tenantId = principal.data?.tenant_id ?? null;
  // A super-admin principal (`*`) must name a tenant explicitly; a normal tenant
  // user is pinned to their own, so the param is omitted.
  const tenantParam = tenantId && tenantId !== "*" ? tenantId : undefined;

  const instances = useQuery({
    queryKey: ["nexus", "dashboard-acl", dashboardId, tenantParam],
    enabled: open,
    queryFn: () =>
      client.listResourceInstances(DASHBOARD_KIND, { tenant: tenantParam }),
  });

  const instance: ResourceInstance | null =
    instances.data?.items.find((i) => i.id === dashboardId) ?? null;

  // The drawer is its own Sheet; `page = null` keeps it closed. We close the
  // dialog by clearing the page through `onClose`.
  return (
    <PageDetailDrawer
      page={open ? instance : null}
      kind={DASHBOARD_KIND}
      tenantId={tenantParam ?? tenantId ?? ""}
      onClose={() => onOpenChange(false)}
    />
  );
}
