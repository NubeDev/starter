// Access management — the "who can do what" surface. Each tab is now a
// real route (deep-linkable, back/forward works):
//   • /access/navigation  — the nav builder (author the tree, grant per node)
//   • /access/nav-manager  — the team×node access matrix
//   • /access/teams        — create/delete teams, manage team members
//   • /access/members      — add/remove tenant members, change their role
//
// `AccessPage` is the layout: it resolves the tenant + user directory once
// and renders the tab bar + an <Outlet/>. The tab bar is the same styled
// Radix Tabs as before, but driven by the URL (value = current segment,
// onValueChange = navigate) so the look is unchanged while the URL leads.
//
// Teams and Members reuse the ready-made CRUD panels from
// `@nube/starter-ui-authz`, wired to this tenant. New *user accounts* are
// created via the Members tab's invite dialog — these panels manage who
// belongs to the tenant and its teams, by user id.

import {
  Outlet,
  useLocation,
  useNavigate,
  useOutletContext,
} from "react-router-dom";
import {
  Tabs,
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
import { NavBuilder } from "@/features/nav/NavBuilderPage";
import { NavManager } from "@/features/nav/NavManager";
import { useMemberDirectory } from "@/features/access/useMemberDirectory";
import { CreateUserForm } from "@/features/access/CreateUserForm";

// Outlet context shared by every Access tab. `scopedTenant` is the single
// tenant the panels manage, or null for a super-admin (`*`) who has no one
// tenant in scope.
interface AccessContext {
  scopedTenant: string | null;
}

function useAccessContext(): AccessContext {
  return useOutletContext<AccessContext>();
}

// The tab segments, in display order. The `value` is the URL segment under
// /access, so the styled tab bar can be driven straight from the path.
const TABS = [
  { value: "navigation", label: "Navigation" },
  { value: "nav-manager", label: "Navigation Manager" },
  { value: "teams", label: "Teams" },
  { value: "members", label: "Members" },
] as const;

export function AccessPage() {
  const principal = usePrincipal();
  const navigate = useNavigate();
  const location = useLocation();

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

  // Which tab is active = the segment after /access (defaults to navigation).
  const active =
    TABS.find((t) => location.pathname.startsWith(`/access/${t.value}`))
      ?.value ?? "navigation";

  return (
    <AccessShell scopedTenant={scopedTenant}>
      <Tabs
        value={active}
        onValueChange={(v) => navigate(`/access/${v}`)}
        className="flex h-full flex-col"
      >
        <TabsList>
          {TABS.map((t) => (
            <TabsTrigger key={t.value} value={t.value}>
              {t.label}
            </TabsTrigger>
          ))}
        </TabsList>
        <div className="mt-4 min-h-0 flex-1 overflow-y-auto">
          <Outlet context={{ scopedTenant } satisfies AccessContext} />
        </div>
      </Tabs>
    </AccessShell>
  );
}

// Wraps the routed tabs in the user-directory provider so the Teams/Members
// pickers can enumerate the tenant's people. Split out so the directory hook
// (which subscribes to the members query) only runs once a tenant is resolved.
function AccessShell({
  scopedTenant,
  children,
}: {
  scopedTenant: string | null;
  children: React.ReactNode;
}) {
  const directory = useMemberDirectory(scopedTenant);
  return (
    <UserDirectoryProvider value={directory}>{children}</UserDirectoryProvider>
  );
}

// ---------------------------------------------------------------- tab routes

// WS-13 §6: access is granted per nav node, not per dashboard. The Navigation
// tab is the full nav builder — authoring the tree and granting access per
// node live in one place.
export function AccessNavigationTab() {
  return (
    <div className="h-full">
      <NavBuilder />
    </div>
  );
}

// Navigation Manager — the sidebar-shaped team×node access matrix. One place
// to see who can reach what, toggle access in bulk, reorder by drag, CRUD nodes.
export function AccessNavManagerTab() {
  const { scopedTenant } = useAccessContext();
  return (
    <div className="h-full">
      <NavManager tenantId={scopedTenant} />
    </div>
  );
}

export function AccessTeamsTab() {
  const { scopedTenant } = useAccessContext();
  return (
    <div className="h-full">
      {scopedTenant ? <TeamsPanel tenantId={scopedTenant} /> : <SuperAdminHint />}
    </div>
  );
}

export function AccessMembersTab() {
  const { scopedTenant } = useAccessContext();
  return (
    <div className="h-full">
      {scopedTenant ? <MembersTab tenantId={scopedTenant} /> : <SuperAdminHint />}
    </div>
  );
}

// Members panel in controlled mode: it has no list endpoint of its own, so the
// host fetches the membership list and passes it in for the row display.
function MembersTab({ tenantId }: { tenantId: string }) {
  const members = useTenantMembers(tenantId);
  return (
    <MembersPanel
      tenantId={tenantId}
      members={members.data}
      membersLoading={members.isLoading}
      membersError={members.error}
      // The "Add member" dialog adds EXISTING users; this slot adds the
      // "invite a brand-new account" path into the same dialog.
      addExtra={<CreateUserForm tenantId={tenantId} embedded />}
    />
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
