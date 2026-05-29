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
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
  ScrollArea,
  Separator,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  cn,
} from "@nube/starter-ui-kit";
import {
  Activity,
  Building2,
  ChevronDown,
  ChevronRight,
  CircleHelp,
  FileSearch,
  Layers,
  Search,
  ShieldCheck,
  UserRound,
  Users,
} from "lucide-react";
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
    <div className="grid gap-6">
      {header}
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div className="grid gap-1">
          <h1 className="text-2xl font-semibold tracking-tight">
            {m.shell.title}
          </h1>
          <p className="text-sm text-muted-foreground">
            Tenants, teams, members, rules, and audit trail.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => setDrawer("resources")}
          >
            <Layers className="size-4" aria-hidden />
            {m.shell.tabs.resources}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setDrawer("check")}
          >
            <ShieldCheck className="size-4" aria-hidden />
            {m.shell.tabs.check}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setDrawer("decisions")}
          >
            <FileSearch className="size-4" aria-hidden />
            {m.shell.tabs.decisions}
          </Button>
        </div>
      </header>

      <div className="grid gap-6 lg:grid-cols-[20rem_1fr]">
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
        <SheetContent
          side="right"
          className="w-full gap-0 sm:max-w-3xl"
        >
          <SheetHeader className="border-b border-border px-6 py-4">
            <SheetTitle className="flex items-center gap-2 text-lg">
              {drawer === "resources" ? (
                <Layers className="size-4 text-muted-foreground" aria-hidden />
              ) : drawer === "check" ? (
                <ShieldCheck
                  className="size-4 text-muted-foreground"
                  aria-hidden
                />
              ) : (
                <FileSearch
                  className="size-4 text-muted-foreground"
                  aria-hidden
                />
              )}
              {drawer === "resources"
                ? m.shell.tabs.resources
                : drawer === "check"
                ? m.shell.tabs.check
                : m.shell.tabs.decisions}
            </SheetTitle>
            <SheetDescription>
              {drawer === "resources"
                ? "Read-only catalogue of resource kinds the engine knows about."
                : drawer === "check"
                ? "Dry-run a principal + action against the live policy."
                : "Recent allow/deny decisions recorded by the engine."}
            </SheetDescription>
          </SheetHeader>
          <ScrollArea className="flex-1">
            <div className="px-6 py-6">
              {drawer === "resources" ? <ResourcesPanel /> : null}
              {drawer === "check" ? <CheckPanel /> : null}
              {drawer === "decisions" ? <DecisionsPanel /> : null}
            </div>
          </ScrollArea>
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
    <Card
      size="sm"
      className="sticky top-4 flex h-fit max-h-[calc(100vh-2rem)] flex-col gap-0 overflow-hidden p-0"
    >
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <button
          type="button"
          className="flex items-center gap-2 text-sm font-semibold tracking-tight hover:text-primary"
          onClick={() => onSelect({ kind: "root" })}
        >
          <Building2 className="size-4 text-muted-foreground" aria-hidden />
          {m.shell.tabs.tenants}
        </button>
        <Badge variant="secondary" className="font-mono text-[10px]">
          {tenants.length}
        </Badge>
      </div>

      <ScrollArea className="flex-1">
        <div className="px-2 py-2">
          {list.isLoading ? (
            <div className="px-3 py-4 text-xs text-muted-foreground">
              {m.common.loading}
            </div>
          ) : list.error ? (
            <div className="grid gap-2 px-3 py-3 text-xs">
              <p className="text-destructive">
                {list.error.message || m.common.error}
              </p>
              <Button
                size="sm"
                variant="outline"
                onClick={() => void list.refetch()}
              >
                {m.common.refresh}
              </Button>
            </div>
          ) : tenants.length === 0 ? (
            <div className="grid gap-2 px-3 py-4 text-xs text-muted-foreground">
              <p>{m.common.empty}</p>
              <Button
                size="sm"
                variant="outline"
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
                const Chevron = isOpen ? ChevronDown : ChevronRight;
                return (
                  <li key={t.id}>
                    <div
                      className={cn(
                        "group flex items-center gap-1 rounded-md px-1.5 py-1 transition-colors",
                        isSel
                          ? "bg-accent text-accent-foreground"
                          : "hover:bg-muted/60",
                      )}
                    >
                      <button
                        type="button"
                        aria-label={isOpen ? "Collapse" : "Expand"}
                        className="grid size-5 place-items-center rounded text-muted-foreground hover:bg-background hover:text-foreground"
                        onClick={() =>
                          setExpanded((e) => ({ ...e, [t.id]: !isOpen }))
                        }
                      >
                        <Chevron className="size-3.5" aria-hidden />
                      </button>
                      <button
                        type="button"
                        className="flex flex-1 items-center gap-2 truncate text-left text-sm"
                        onClick={() =>
                          onSelect({ kind: "tenant", tenantId: t.id })
                        }
                        title={t.display_name}
                      >
                        <span className="truncate font-medium">
                          {t.display_name}
                        </span>
                        <code className="ml-auto text-[10px] text-muted-foreground">
                          {t.slug}
                        </code>
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
      </ScrollArea>

      <div className="border-t border-border p-2">
        <div className="relative">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Search tenants…"
            aria-label="Search tenants"
            className="pl-8"
          />
        </div>
      </div>
    </Card>
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
    <ul className="ml-4 grid gap-0.5 border-l border-border pl-2 py-1">
      <li>
        <div className="flex items-center gap-1.5 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          <Users className="size-3" aria-hidden />
          {m.shell.tabs.teams}
        </div>
        {teams.isLoading ? (
          <div className="px-2 py-1 text-xs text-muted-foreground">
            {m.common.loading}
          </div>
        ) : teams.error ? (
          <div className="px-2 py-1 text-xs text-destructive">
            {teams.error.message}
          </div>
        ) : (teams.data ?? []).length === 0 ? (
          <div className="px-2 py-1 text-xs text-muted-foreground">—</div>
        ) : (
          <ul className="grid gap-0.5">
            {(teams.data ?? []).map((tm) => {
              const isSel = selectedTeamId === tm.id;
              return (
                <li key={tm.id}>
                  <button
                    type="button"
                    className={cn(
                      "flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm transition-colors",
                      isSel
                        ? "bg-accent text-accent-foreground"
                        : "hover:bg-muted/60",
                    )}
                    onClick={() =>
                      onSelect({ kind: "team", tenantId, teamId: tm.id })
                    }
                  >
                    <span className="truncate">{tm.display_name}</span>
                    <code className="ml-auto text-[10px] text-muted-foreground">
                      {tm.slug}
                    </code>
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
          className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
          onClick={() => onSelect({ kind: "tenant", tenantId })}
        >
          <UserRound className="size-3" aria-hidden />
          {m.shell.tabs.members}
        </button>
      </li>
      <li>
        <button
          type="button"
          className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
          onClick={() => onSelect({ kind: "tenant", tenantId })}
        >
          <UserRound className="size-3" aria-hidden />
          Users
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
      <TabsList>
        <TabsTrigger value="overview">
          <Activity className="size-4" aria-hidden />
          Overview
        </TabsTrigger>
        <TabsTrigger value="tenants">
          <Building2 className="size-4" aria-hidden />
          {m.shell.tabs.tenants}
        </TabsTrigger>
      </TabsList>
      <TabsContent value="overview" className="mt-6">
        <Card>
          <CardHeader className="border-b">
            <CardTitle className="flex items-center gap-2">
              <CircleHelp className="size-4 text-muted-foreground" aria-hidden />
              Getting started
            </CardTitle>
            <CardDescription>
              Access Control is scoped to whatever you pick in the left rail.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-4 text-sm">
            <div className="grid gap-3 sm:grid-cols-3">
              <RailHint
                icon={Building2}
                title="Pick a tenant"
                body="Manage its teams, members, rules, and assignments from the right pane."
              />
              <RailHint
                icon={Layers}
                title="Browse resources"
                body="Use the Resources button to inspect what kinds the engine knows about."
              />
              <RailHint
                icon={ShieldCheck}
                title="Dry-run a check"
                body="Use Check to test a (principal, action, resource) without writing a rule."
              />
            </div>
            <Separator />
            <p className="text-muted-foreground">
              The toolbar buttons (Resources, Check, Decisions) stay global —
              they open in a side panel and never close your tenant context.
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

function RailHint({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof Building2;
  title: string;
  body: string;
}) {
  return (
    <div className="grid gap-2 rounded-xl border border-border bg-muted/30 p-4">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Icon className="size-4 text-muted-foreground" aria-hidden />
        {title}
      </div>
      <p className="text-xs leading-relaxed text-muted-foreground">{body}</p>
    </div>
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
      <TabsList>
        <TabsTrigger value="overview">
          <Activity className="size-4" aria-hidden />
          Overview
        </TabsTrigger>
        <TabsTrigger value="teams">
          <Users className="size-4" aria-hidden />
          {m.shell.tabs.teams}
        </TabsTrigger>
        <TabsTrigger value="members">
          <UserRound className="size-4" aria-hidden />
          {m.shell.tabs.members}
        </TabsTrigger>
        <TabsTrigger value="rules">
          <ShieldCheck className="size-4" aria-hidden />
          {m.shell.tabs.rules}
        </TabsTrigger>
        <TabsTrigger value="assignments">{m.shell.tabs.assignments}</TabsTrigger>
        <TabsTrigger value="decisions">
          <FileSearch className="size-4" aria-hidden />
          {m.shell.tabs.decisions}
        </TabsTrigger>
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
