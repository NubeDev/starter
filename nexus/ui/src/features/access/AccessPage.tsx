// Access management — the "who can do what" surface, in three tabs:
//   • Dashboards — per-dashboard sharing (share scope + grants)
//   • Teams      — create/delete teams, add members (reused authz panel)
//   • Members    — add/remove tenant members, change their role
//
// Teams and Members reuse the ready-made CRUD panels from
// `@nube/starter-ui-authz` (the same ones the standalone authz admin uses),
// wired to this tenant. New *user accounts* are created via sign-up/CLI — these
// panels manage who belongs to the tenant and its teams, by user id.

import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";
import {
  MembersPanel,
  TeamsPanel,
  UserDirectoryProvider,
  useTenantMembers,
} from "@nube/starter-ui-authz";

import { usePrincipal } from "@/auth/usePrincipal";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { NavAccessTab } from "@/features/access/NavAccessTab";
import { useMemberDirectory } from "@/features/access/useMemberDirectory";
import { CreateUserForm } from "@/features/access/CreateUserForm";

export function AccessPage() {
  const principal = usePrincipal();

  if (principal.isPending) return <Loading label="Loading…" />;
  if (principal.isError) {
    return (
      <ErrorState
        message={
          principal.error instanceof Error ? principal.error.message : undefined
        }
      />
    );
  }

  const tenantId = principal.data?.tenant_id ?? null;
  // The team/member panels are tenant-scoped; a super-admin (`*`) has no single
  // tenant to manage here, so we tell them to pick one rather than silently fail.
  const scopedTenant = tenantId && tenantId !== "*" ? tenantId : null;

  return (
    <AccessTabs scopedTenant={scopedTenant} />
  );
}

// Split out so the member directory hook (which subscribes to the members query)
// only runs once a tenant is resolved, and the picker provider wraps both panels.
function AccessTabs({ scopedTenant }: { scopedTenant: string | null }) {
  const directory = useMemberDirectory(scopedTenant);

  return (
    <UserDirectoryProvider value={directory}>
    <Tabs defaultValue="navigation" className="flex h-full flex-col">
      <TabsList>
        <TabsTrigger value="navigation">Navigation</TabsTrigger>
        <TabsTrigger value="teams">Teams</TabsTrigger>
        <TabsTrigger value="members">Members</TabsTrigger>
      </TabsList>
      <div className="mt-4 min-h-0 flex-1 overflow-y-auto">
        {/* WS-13 §6: access is granted per nav node, not per dashboard — the
            Navigation tab replaces the old Dashboards tab. A node grant gives a
            user a specific mount, not every page that reuses the template. */}
        <TabsContent value="navigation" className="h-full">
          <NavAccessTab tenantId={scopedTenant} />
        </TabsContent>
        <TabsContent value="teams" className="h-full">
          {scopedTenant ? (
            <TeamsPanel tenantId={scopedTenant} />
          ) : (
            <SuperAdminHint />
          )}
        </TabsContent>
        <TabsContent value="members" className="h-full">
          {scopedTenant ? (
            <MembersTab tenantId={scopedTenant} />
          ) : (
            <SuperAdminHint />
          )}
        </TabsContent>
      </div>
    </Tabs>
    </UserDirectoryProvider>
  );
}

// Members panel in controlled mode: it has no list endpoint of its own, so the
// host fetches the membership list and passes it in for the row display.
function MembersTab({ tenantId }: { tenantId: string }) {
  const members = useTenantMembers(tenantId);
  return (
    <div className="grid gap-6">
      <CreateUserForm tenantId={tenantId} />
      <MembersPanel
        tenantId={tenantId}
        members={members.data}
        membersLoading={members.isLoading}
        membersError={members.error}
      />
    </div>
  );
}

function SuperAdminHint() {
  return (
    <p className="text-sm text-muted-foreground">
      You are signed in as a global admin. Team and member management is
      per-tenant — sign in as a member of a single tenant to manage it.
    </p>
  );
}
