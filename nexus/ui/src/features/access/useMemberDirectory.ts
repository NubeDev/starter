// Builds a UserDirectory (the searchable people-source the authz UserPicker
// consumes) from the tenant's members. This turns the "type a user id" fallback
// in the Teams/Members panels into a real dropdown of the tenant's people,
// labelled by email.
//
// Source = GET /v1/tenants/{id}/members (now returns email). The picker only
// offers people who already belong to the tenant — which is exactly who you can
// add to a team. Adding a brand-new person to the tenant is the Members tab's
// own add form.

import { useMemo } from "react";
import type { UserDirectory } from "@nube/starter-ui-authz";

import { useTenantMembers } from "@nube/starter-ui-authz";

/** A UserDirectory backed by the tenant's member list, or null while it loads. */
export function useMemberDirectory(tenantId: string | null): UserDirectory | null {
  const members = useTenantMembers(tenantId);

  return useMemo<UserDirectory | null>(() => {
    const rows = members.data;
    if (!rows) return null;

    const entries = rows.map((m) => ({
      user_id: m.user_id,
      email: m.email ?? m.user_id,
      role: m.role,
    }));

    return {
      search(query: string) {
        const q = query.trim().toLowerCase();
        if (!q) return entries;
        return entries.filter(
          (e) =>
            e.email.toLowerCase().includes(q) ||
            e.user_id.toLowerCase().includes(q),
        );
      },
      getById(id: string) {
        return entries.find((e) => e.user_id === id);
      },
    };
  }, [members.data]);
}
