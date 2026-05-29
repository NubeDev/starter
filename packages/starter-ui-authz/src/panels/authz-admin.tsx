// `<AuthzAdmin>` — master-detail shell. Left rail is a tenant ->
// team -> user tree; right pane shows scoped tabs derived from
// `selectedNode`. Global tools (Resources, Check, Decisions) live
// in a top-right toolbar that opens them in a `<Sheet>` drawer.
//
// Selection is *controlled* — the host (rubix's `/admin/access`
// route) owns URL <-> state mapping and feeds us `selectedNode` +
// `onSelectNode`. When the host doesn't wire that up we fall back
// to local state so the shell still works in isolation.

import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit";
import { AuthzI18nProvider } from "../i18n/context.js";
import type { AuthzMessages } from "../i18n/messages.js";
import { useAuthzMessages } from "../i18n/context.js";
import { useTenants, useTeams, useAddTeamMember } from "../hooks/index.js";
import { TenantsPanel } from "./tenants-panel.js";
import { MembersPanel } from "./members-panel.js";
import { TeamsPanel } from "./teams-panel.js";
import { RulesPanel } from "./rules-panel.js";
import { AssignmentsPanel } from "./assignments-panel.js";
import { ResourcesPanel } from "./resources-panel.js";
import { CheckPanel } from "./check-panel.js";
import { DecisionsPanel } from "./decisions-panel.js";
import {
  UserDirectoryProvider,
  UserPicker,
  UserPickerFallback,
  useUserDirectory,
  type UserDirectory,
} from "./user-picker.js";
import { UserOpsProvider, useUserOps, type UsersAdminOps } from "./user-ops.js";
import { UserProfilePanel } from "./user-profile-panel.js";
import { StateRow } from "./_common.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type SelectedNode =
  | { kind: "root" }
  | { kind: "tenant"; tenantId: string }
  | { kind: "team"; tenantId: string; teamId: string }
  | { kind: "user"; userId: string; tenantId?: string };

/** Legacy tab union. Retained as an export so older callers keep
 *  compiling; the new shell ignores it. */
export type AuthzAdminTab =
  | "tenants"
  | "teams"
  | "members"
  | "rules"
  | "assignments"
  | "resources"
  | "check"
  | "decisions";

/** Plug-in slot for Agent C's `/admin/users` -> Profile migration. */
export interface UserDetailExtras {
  /** Rendered as the "Profile" tab body on the user detail pane. */
  renderProfile?: (ctx: { userId: string; tenantId?: string }) => ReactNode;
}

export interface AuthzAdminProps {
  /** Optional i18n override merged on top of `DEFAULT_AUTHZ_MESSAGES`. */
  i18n?: Partial<AuthzMessages>;
  /** Optional slot rendered above the shell (page header). */
  header?: ReactNode;
  /** Host-supplied user directory powering `<UserPicker>`. */
  userDirectory?: UserDirectory;
  /** Host-supplied user ops adapter — exposed to children via
   *  `<UserOpsProvider>` so Agent C's `UserProfilePanel` /
   *  `UsersListPanel` can plug into the shell without depending on
   *  `@nube/rubix-client-react`. */
  userOps?: UsersAdminOps;
  /** When true, `<UserPicker>` exposes the Team segment. */
  enableTeamMode?: boolean;
  /** Controlled selection. When omitted the shell tracks selection internally. */
  selectedNode?: SelectedNode;
  /** Notified whenever the rail or a deep-link button changes selection. */
  onSelectNode?: (next: SelectedNode) => void;
  /** Default selection used when uncontrolled. */
  defaultSelectedNode?: SelectedNode;
  /** Profile tab renderer for the user detail pane (see UserDetailExtras). */
  userDetailExtras?: UserDetailExtras;
  /** @deprecated kept for backward compatibility; ignored by the new shell. */
  defaultTab?: AuthzAdminTab;
  /** @deprecated retained for callers — initial tenant selection. */
  initialTenantId?: string;
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

export function AuthzAdmin(props: AuthzAdminProps) {
  return (
    <AuthzI18nProvider value={props.i18n}>
      <UserDirectoryProvider value={props.userDirectory}>
        <UserOpsProvider value={props.userOps}>
          <AuthzAdminInner {...props} />
        </UserOpsProvider>
      </UserDirectoryProvider>
    </AuthzI18nProvider>
  );
}

function AuthzAdminInner({
  header,
  enableTeamMode,
  selectedNode,
  onSelectNode,
  defaultSelectedNode,
  userDetailExtras,
  initialTenantId,
}: AuthzAdminProps) {
  const m = useAuthzMessages();

  // Uncontrolled fallback so the package works without a router host.
  const [localSel, setLocalSel] = useState<SelectedNode>(
    defaultSelectedNode ??
      (initialTenantId
        ? { kind: "tenant", tenantId: initialTenantId }
        : { kind: "root" }),
  );
  const sel = selectedNode ?? localSel;
  const setSel = (next: SelectedNode) => {
    if (onSelectNode) onSelectNode(next);
    else setLocalSel(next);
  };

  const [drawer, setDrawer] = useState<null | "resources" | "check" | "decisions">(null);

  return (
    <div className="grid gap-4">
      {header}
      <header className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">{m.shell.title}</h1>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" onClick={() => setDrawer("resources")}>
            {m.shell.tabs.resources}
          </Button>
          <Button size="sm" variant="outline" onClick={() => setDrawer("check")}>
            {m.shell.tabs.check}
          </Button>
          <Button size="sm" variant="outline" onClick={() => setDrawer("decisions")}>
            {m.shell.tabs.decisions}
          </Button>
        </div>
      </header>

      <div className="grid gap-6 lg:grid-cols-[18rem_1fr]">
        <TenantRail
          selected={sel}
          onSelect={setSel}
          enableTeamMode={!!enableTeamMode}
        />
        <div className="min-w-0">
          <DetailPane
            sel={sel}
            onSelect={setSel}
            enableTeamMode={!!enableTeamMode}
            userDetailExtras={userDetailExtras}
          />
        </div>
      </div>

      <Sheet open={drawer !== null} onOpenChange={(o) => !o && setDrawer(null)}>
        <SheetContent side="right" className="w-full sm:max-w-3xl overflow-y-auto">
          <SheetHeader>
            <SheetTitle>
              {drawer === "resources"
                ? m.shell.tabs.resources
                : drawer === "check"
                ? m.shell.tabs.check
                : m.shell.tabs.decisions}
            </SheetTitle>
          </SheetHeader>
          <div className="mt-4">
            {drawer === "resources" ? <ResourcesPanel /> : null}
            {drawer === "check" ? <CheckPanel /> : null}
            {drawer === "decisions" ? <DecisionsPanel /> : null}
          </div>
        </SheetContent>
      </Sheet>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Left rail
// ---------------------------------------------------------------------------

function TenantRail({
  selected,
  onSelect,
}: {
  selected: SelectedNode;
  onSelect: (n: SelectedNode) => void;
  enableTeamMode: boolean;
}) {
  const m = useAuthzMessages();
  const list = useTenants();
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [query, setQuery] = useState("");

  const selectedTenantId =
    selected.kind === "tenant" || selected.kind === "team"
      ? selected.tenantId
      : selected.kind === "user"
      ? selected.tenantId
      : undefined;

  // Auto-expand the tenant we are scoped to.
  useEffect(() => {
    if (selectedTenantId) setExpanded((e) => ({ ...e, [selectedTenantId]: true }));
  }, [selectedTenantId]);

  const tenants = useMemo(() => {
    const all = list.data ?? [];
    const q = query.trim().toLowerCase();
    if (!q) return all;
    return all.filter(
      (t) => t.slug.toLowerCase().includes(q) || t.display_name.toLowerCase().includes(q),
    );
  }, [list.data, query]);

  return (
    <aside className="flex h-full flex-col rounded-2xl border border-[color:var(--color-border,#e5e7eb)] bg-[color:var(--color-surface,#fff)]/40">
      <div className="flex items-center justify-between border-b border-[color:var(--color-border,#e5e7eb)] px-3 py-2">
        <button
          type="button"
          className="text-sm font-semibold tracking-tight hover:underline"
          onClick={() => onSelect({ kind: "root" })}
        >
          {m.shell.tabs.tenants}
        </button>
        <span className="text-xs opacity-60">{tenants.length}</span>
      </div>

      <div className="flex-1 overflow-y-auto px-1 py-2">
        {list.isLoading ? (
          <div className="px-3 py-4 text-xs opacity-60">{m.common.loading}</div>
        ) : list.error ? (
          // /v1/tenants is known broken in some envs — show a friendly,
          // recoverable error instead of throwing.
          <div className="px-3 py-3 text-xs">
            <p className="text-[color:var(--color-danger,#dc2626)]">
              {list.error.message || m.common.error}
            </p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => void list.refetch()}
            >
              {m.common.refresh}
            </Button>
          </div>
        ) : tenants.length === 0 ? (
          <div className="px-3 py-4 text-xs opacity-70">
            <p>{m.common.empty}</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => onSelect({ kind: "root" })}
            >
              {m.tenants.form.submit}
            </Button>
          </div>
        ) : (
          <ul className="grid gap-0.5">
            {tenants.map((t) => {
              const isOpen = expanded[t.id] ?? false;
              const isSel = selectedTenantId === t.id;
              return (
                <li key={t.id}>
                  <div
                    className={
                      isSel
                        ? "flex items-center gap-1 rounded-md bg-[color:var(--color-accent-soft,#eef2ff)] px-2 py-1"
                        : "flex items-center gap-1 rounded-md px-2 py-1 hover:bg-[color:var(--color-muted,#f9fafb)]"
                    }
                  >
                    <button
                      type="button"
                      aria-label={isOpen ? "Collapse" : "Expand"}
                      className="grid h-4 w-4 place-items-center text-xs opacity-60"
                      onClick={() => setExpanded((e) => ({ ...e, [t.id]: !isOpen }))}
                    >
                      {isOpen ? "▾" : "▸"}
                    </button>
                    <button
                      type="button"
                      className="flex-1 truncate text-left text-sm"
                      onClick={() => onSelect({ kind: "tenant", tenantId: t.id })}
                      title={t.display_name}
                    >
                      <span className="font-medium">{t.display_name}</span>
                      <span className="ml-2 text-xs opacity-60">
                        <code>{t.slug}</code>
                      </span>
                    </button>
                  </div>
                  {isOpen ? (
                    <TenantChildren
                      tenantId={t.id}
                      selected={selected}
                      onSelect={onSelect}
                    />
                  ) : null}
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <div className="border-t border-[color:var(--color-border,#e5e7eb)] p-2">
        <Input
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          placeholder="Search tenants…"
          aria-label="Search tenants"
        />
      </div>
    </aside>
  );
}

function TenantChildren({
  tenantId,
  selected,
  onSelect,
}: {
  tenantId: string;
  selected: SelectedNode;
  onSelect: (n: SelectedNode) => void;
}) {
  const m = useAuthzMessages();
  const teams = useTeams(tenantId);
  const selectedTeamId = selected.kind === "team" ? selected.teamId : undefined;

  return (
    <ul className="ml-5 grid gap-0.5 border-l border-[color:var(--color-border,#e5e7eb)] pl-2 py-1">
      <li>
        <div className="px-2 py-0.5 text-[11px] uppercase tracking-wider opacity-60">
          {m.shell.tabs.teams}
        </div>
        {teams.isLoading ? (
          <div className="px-2 py-1 text-xs opacity-60">{m.common.loading}</div>
        ) : teams.error ? (
          <div className="px-2 py-1 text-xs text-[color:var(--color-danger,#dc2626)]">
            {teams.error.message}
          </div>
        ) : (teams.data ?? []).length === 0 ? (
          <div className="px-2 py-1 text-xs opacity-60">—</div>
        ) : (
          <ul>
            {(teams.data ?? []).map((tm) => {
              const isSel = selectedTeamId === tm.id;
              return (
                <li key={tm.id}>
                  <button
                    type="button"
                    className={
                      isSel
                        ? "block w-full rounded-md bg-[color:var(--color-accent-soft,#eef2ff)] px-2 py-1 text-left text-sm"
                        : "block w-full rounded-md px-2 py-1 text-left text-sm hover:bg-[color:var(--color-muted,#f9fafb)]"
                    }
                    onClick={() =>
                      onSelect({ kind: "team", tenantId, teamId: tm.id })
                    }
                  >
                    <span>{tm.display_name}</span>
                    <span className="ml-2 text-xs opacity-60">
                      <code>{tm.slug}</code>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </li>
      <li>
        <button
          type="button"
          className="block w-full rounded-md px-2 py-1 text-left text-sm hover:bg-[color:var(--color-muted,#f9fafb)]"
          onClick={() => onSelect({ kind: "tenant", tenantId })}
        >
          <span className="text-[11px] uppercase tracking-wider opacity-60">
            {m.shell.tabs.members}
          </span>
        </button>
      </li>
      <li>
        {/* "Users" sub-node — Agent C populates the actual list. */}
        <button
          type="button"
          className="block w-full rounded-md px-2 py-1 text-left text-sm hover:bg-[color:var(--color-muted,#f9fafb)]"
          onClick={() => onSelect({ kind: "tenant", tenantId })}
        >
          <span className="text-[11px] uppercase tracking-wider opacity-60">
            Users
          </span>
        </button>
      </li>
    </ul>
  );
}

// ---------------------------------------------------------------------------
// Right pane
// ---------------------------------------------------------------------------

function DetailPane({
  sel,
  onSelect,
  enableTeamMode,
  userDetailExtras,
}: {
  sel: SelectedNode;
  onSelect: (n: SelectedNode) => void;
  enableTeamMode: boolean;
  userDetailExtras?: UserDetailExtras;
}) {
  if (sel.kind === "root") return <RootDetail onSelect={onSelect} />;
  if (sel.kind === "tenant")
    return <TenantDetail tenantId={sel.tenantId} enableTeamMode={enableTeamMode} />;
  if (sel.kind === "team")
    return (
      <TeamDetail
        tenantId={sel.tenantId}
        teamId={sel.teamId}
        enableTeamMode={enableTeamMode}
      />
    );
  return <UserDetail sel={sel} extras={userDetailExtras} />;
}

function RootDetail({ onSelect }: { onSelect: (n: SelectedNode) => void }) {
  const m = useAuthzMessages();
  return (
    <Tabs defaultValue="overview">
      <TabsList className="flex-wrap">
        <TabsTrigger value="overview">Overview</TabsTrigger>
        <TabsTrigger value="tenants">{m.shell.tabs.tenants}</TabsTrigger>
      </TabsList>
      <TabsContent value="overview" className="mt-6">
        <Card>
          <CardHeader>
            <CardTitle>Access Control</CardTitle>
          </CardHeader>
          <CardContent className="text-sm opacity-80">
            <p>
              Pick a tenant in the left rail to manage its teams, members,
              rules, and assignments. Use the toolbar buttons for the global
              Resources catalog, dry-run Check, and Decisions audit feed.
            </p>
          </CardContent>
        </Card>
      </TabsContent>
      <TabsContent value="tenants" className="mt-6">
        <TenantsPanel
          onSelectTenant={(id) => onSelect({ kind: "tenant", tenantId: id })}
        />
      </TabsContent>
    </Tabs>
  );
}

function TenantDetail({
  tenantId,
  enableTeamMode,
}: {
  tenantId: string;
  enableTeamMode: boolean;
}) {
  const m = useAuthzMessages();
  return (
    <Tabs defaultValue="overview">
      <TabsList className="flex-wrap">
        <TabsTrigger value="overview">Overview</TabsTrigger>
        <TabsTrigger value="teams">{m.shell.tabs.teams}</TabsTrigger>
        <TabsTrigger value="members">{m.shell.tabs.members}</TabsTrigger>
        <TabsTrigger value="rules">{m.shell.tabs.rules}</TabsTrigger>
        <TabsTrigger value="assignments">{m.shell.tabs.assignments}</TabsTrigger>
        <TabsTrigger value="decisions">{m.shell.tabs.decisions}</TabsTrigger>
      </TabsList>
      <TabsContent value="overview" className="mt-6">
        <TenantOverview tenantId={tenantId} />
      </TabsContent>
      <TabsContent value="teams" className="mt-6">
        <TeamsPanel tenantId={tenantId} />
      </TabsContent>
      <TabsContent value="members" className="mt-6">
        <MembersPanel tenantId={tenantId} />
      </TabsContent>
      <TabsContent value="rules" className="mt-6">
        <RulesPanel tenantId={tenantId} />
      </TabsContent>
      <TabsContent value="assignments" className="mt-6">
        <AssignmentsPanel tenantId={tenantId} enableTeamMode={enableTeamMode} />
      </TabsContent>
      <TabsContent value="decisions" className="mt-6">
        <DecisionsPanel tenantId={tenantId} />
      </TabsContent>
    </Tabs>
  );
}

function TenantOverview({ tenantId }: { tenantId: string }) {
  const tenants = useTenants();
  const t = (tenants.data ?? []).find((x) => x.id === tenantId);
  if (!t) return <StateRow variant="empty">Tenant not found.</StateRow>;
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.display_name}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-1 text-sm">
        <div>
          <span className="opacity-60">Slug:</span> <code>{t.slug}</code>
        </div>
        <div>
          <span className="opacity-60">ID:</span>{" "}
          <code className="text-xs">{t.id}</code>
        </div>
        <div>
          <span className="opacity-60">Audit sample:</span>{" "}
          {t.audit_allow_sample ?? "—"}
        </div>
      </CardContent>
    </Card>
  );
}

function TeamDetail({
  tenantId,
  teamId,
  enableTeamMode,
}: {
  tenantId: string;
  teamId: string;
  enableTeamMode: boolean;
}) {
  const m = useAuthzMessages();
  const teams = useTeams(tenantId);
  const team = (teams.data ?? []).find((x) => x.id === teamId);
  const teamSubject = team ? `team:${team.slug}` : null;

  return (
    <Tabs defaultValue="overview">
      <TabsList className="flex-wrap">
        <TabsTrigger value="overview">Overview</TabsTrigger>
        <TabsTrigger value="members">{m.shell.tabs.members}</TabsTrigger>
        <TabsTrigger value="rules">{m.shell.tabs.rules}</TabsTrigger>
        <TabsTrigger value="assignments">{m.shell.tabs.assignments}</TabsTrigger>
      </TabsList>
      <TabsContent value="overview" className="mt-6">
        <TeamOverview tenantId={tenantId} teamId={teamId} />
      </TabsContent>
      <TabsContent value="members" className="mt-6">
        <TeamMembersInline tenantId={tenantId} teamId={teamId} />
      </TabsContent>
      <TabsContent value="rules" className="mt-6">
        <RulesPanel tenantId={tenantId} />
      </TabsContent>
      <TabsContent value="assignments" className="mt-6">
        <AssignmentsPanel
          tenantId={tenantId}
          enableTeamMode={enableTeamMode}
          defaultSubject={teamSubject}
        />
      </TabsContent>
    </Tabs>
  );
}

function TeamOverview({ tenantId, teamId }: { tenantId: string; teamId: string }) {
  const teams = useTeams(tenantId);
  const t = (teams.data ?? []).find((x) => x.id === teamId);
  if (!t) return <StateRow variant="empty">Team not found.</StateRow>;
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.display_name}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-1 text-sm">
        <div>
          <span className="opacity-60">Slug:</span> <code>{t.slug}</code>
        </div>
        <div>
          <span className="opacity-60">Team ID:</span>{" "}
          <code className="text-xs">{t.id}</code>
        </div>
      </CardContent>
    </Card>
  );
}

function TeamMembersInline({
  tenantId,
  teamId,
}: {
  tenantId: string;
  teamId: string;
}) {
  // No `GET /v1/tenants/{id}/teams/{tid}/members` endpoint — we expose
  // the add-member form using the picker; listing is TODO when the API
  // arrives.
  const m = useAuthzMessages();
  const directory = useUserDirectory();
  const addMember = useAddTeamMember();
  const [userId, setUserId] = useState("");

  return (
    <section className="grid gap-4">
      <Card>
        <CardHeader>
          <CardTitle>{m.teams.teamMembers.title}</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!userId.trim()) return;
              void addMember
                .mutateAsync({ tenantId, teamId, body: { user_id: userId.trim() } })
                .then(() => setUserId(""));
            }}
            className="grid grid-cols-1 gap-3 sm:grid-cols-[2fr_auto] sm:items-end"
          >
            <div className="grid gap-1">
              <Label htmlFor="team-add-user">{m.teams.teamMembers.userIdLabel}</Label>
              {directory ? (
                <UserPicker
                  id="team-add-user"
                  value={userId || null}
                  onChange={(sel) => setUserId(sel?.kind === "user" ? sel.id : "")}
                  userDirectory={directory}
                  enableGlobMode={false}
                  placeholder={m.teams.teamMembers.userIdLabel}
                />
              ) : (
                <UserPickerFallback
                  id="team-add-user"
                  value={userId}
                  onChange={setUserId}
                  placeholder={m.teams.teamMembers.userIdLabel}
                />
              )}
            </div>
            <Button type="submit" disabled={addMember.isPending}>
              {m.teams.teamMembers.add}
            </Button>
          </form>
          {addMember.error ? (
            <p className="mt-2 text-xs text-[color:var(--color-danger,#dc2626)]">
              {addMember.error.message}
            </p>
          ) : null}
        </CardContent>
      </Card>
      <StateRow variant="empty">
        Team membership listing is not exposed by the API yet.
      </StateRow>
    </section>
  );
}

function UserProfileSlot({
  sel,
  extras,
}: {
  sel: Extract<SelectedNode, { kind: "user" }>;
  extras?: UserDetailExtras;
}) {
  const ops = useUserOps();
  if (extras?.renderProfile)
    return <>{extras.renderProfile({ userId: sel.userId, tenantId: sel.tenantId })}</>;
  if (ops) return <UserProfilePanel userId={sel.userId} userOps={ops} />;
  return (
    <Card>
      <CardHeader>
        <CardTitle>Profile</CardTitle>
      </CardHeader>
      <CardContent className="text-sm opacity-70">
        Profile (TBD — wire `userOps` or `userDetailExtras.renderProfile`).
      </CardContent>
    </Card>
  );
}

function UserDetail({
  sel,
  extras,
}: {
  sel: Extract<SelectedNode, { kind: "user" }>;
  extras?: UserDetailExtras;
}) {
  const m = useAuthzMessages();
  const subject = sel.userId;
  return (
    <Tabs defaultValue="profile">
      <TabsList className="flex-wrap">
        <TabsTrigger value="profile">Profile</TabsTrigger>
        <TabsTrigger value="memberships">Memberships</TabsTrigger>
        <TabsTrigger value="assignments">{m.shell.tabs.assignments}</TabsTrigger>
        <TabsTrigger value="decisions">{m.shell.tabs.decisions}</TabsTrigger>
      </TabsList>
      <TabsContent value="profile" className="mt-6">
        <UserProfileSlot sel={sel} extras={extras} />
      </TabsContent>
      <TabsContent value="memberships" className="mt-6">
        {/* No per-user memberships endpoint today. Stub until one exists. */}
        <StateRow variant="empty">
          Per-user memberships listing is not exposed by the API yet.
        </StateRow>
      </TabsContent>
      <TabsContent value="assignments" className="mt-6">
        <AssignmentsPanel
          tenantId={sel.tenantId ?? null}
          defaultSubject={subject}
        />
      </TabsContent>
      <TabsContent value="decisions" className="mt-6">
        <DecisionsPanel subject={subject} tenantId={sel.tenantId ?? null} />
      </TabsContent>
    </Tabs>
  );
}
