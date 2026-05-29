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
  Progress,
  ScrollArea,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  cn,
} from "@nube/starter-ui-kit";
import {
  Activity,
  Building2,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  FileSearch,
  Layers,
  Pencil,
  Search,
  ShieldCheck,
  ShieldX,
  UserRound,
  Users,
} from "lucide-react";
import { AuthzI18nProvider } from "../i18n/context.js";
import type { AuthzMessages } from "../i18n/messages.js";
import { useAuthzMessages } from "../i18n/context.js";
import {
  useTenants,
  useTeams,
  useAddTeamMember,
  useTenantMembers,
  useAuthzRules,
  useAuthzDecisions,
} from "../hooks/index.js";
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
      {!header && (
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
      )}

      <DetailPane
        sel={sel}
        onSelect={setSel}
        enableTeamMode={!!enableTeamMode}
        userDetailExtras={userDetailExtras}
      />

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
        <GlobalOverview onSelect={onSelect} />
      </TabsContent>
      <TabsContent value="tenants" className="mt-6">
        <TenantsPanel
          onSelectTenant={(id) => onSelect({ kind: "tenant", tenantId: id })}
        />
      </TabsContent>
    </Tabs>
  );
}

// ---------------------------------------------------------------------------
// Overview — shared visual layout for the root and tenant-scoped Overview tabs
// ---------------------------------------------------------------------------

interface MemberRow {
  user_id: string;
  email: string;
  tenant_id: string;
  tenant_name: string;
  role: string;
}

function StatCard({
  label,
  value,
  hint,
  loading,
  icon: Icon,
  tone,
}: {
  label: string;
  value: string | number;
  hint?: ReactNode;
  loading?: boolean;
  icon: typeof Users;
  tone?: "default" | "success" | "warning";
}) {
  const toneCls =
    tone === "success"
      ? "text-emerald-600 dark:text-emerald-400"
      : tone === "warning"
      ? "text-amber-600 dark:text-amber-400"
      : "text-muted-foreground";
  return (
    <Card className="overflow-hidden">
      <CardContent className="grid gap-2 p-5">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            {label}
          </span>
          <Icon className="size-4 text-muted-foreground" aria-hidden />
        </div>
        <div className="flex items-baseline gap-2">
          {loading ? (
            <Skeleton className="h-8 w-16" />
          ) : (
            <span className="text-3xl font-semibold tracking-tight">{value}</span>
          )}
        </div>
        {hint ? <span className={cn("text-xs", toneCls)}>{hint}</span> : null}
      </CardContent>
    </Card>
  );
}

function Initials({ email }: { email: string }) {
  const seed = (email || "?").trim();
  const ch = seed.slice(0, 2).toUpperCase();
  // Stable hue from email — keeps colors consistent without picking a fixed palette token.
  let h = 0;
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) >>> 0;
  const hue = h % 360;
  return (
    <span
      className="grid size-8 place-items-center rounded-full text-[11px] font-semibold text-white"
      style={{ backgroundColor: `hsl(${hue} 55% 45%)` }}
      aria-hidden
    >
      {ch}
    </span>
  );
}

function MembersTable({
  rows,
  loading,
  showTenant,
  onSelectUser,
  emptyHint,
  footer,
}: {
  rows: MemberRow[];
  loading?: boolean;
  showTenant?: boolean;
  onSelectUser?: (userId: string, tenantId: string) => void;
  emptyHint: string;
  footer?: ReactNode;
}) {
  if (loading) {
    return (
      <div className="grid gap-2 p-6">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-2/3" />
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="grid place-items-center gap-1 px-6 py-10 text-center">
        <Users className="size-5 text-muted-foreground" aria-hidden />
        <p className="text-sm text-muted-foreground">{emptyHint}</p>
      </div>
    );
  }
  return (
    <>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Member</TableHead>
            {showTenant ? <TableHead>Tenant</TableHead> : null}
            <TableHead>Role</TableHead>
            <TableHead className="w-12 text-right" aria-label="Actions" />
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((r) => (
            <TableRow
              key={`${r.tenant_id}:${r.user_id}`}
              className={cn(
                "group",
                onSelectUser && "cursor-pointer",
              )}
              onClick={
                onSelectUser
                  ? () => onSelectUser(r.user_id, r.tenant_id)
                  : undefined
              }
            >
              <TableCell>
                <div className="flex items-center gap-3">
                  <Initials email={r.email} />
                  <div className="grid">
                    <span className="text-sm font-medium">{r.email}</span>
                    <code className="text-[10px] text-muted-foreground">
                      {r.user_id}
                    </code>
                  </div>
                </div>
              </TableCell>
              {showTenant ? (
                <TableCell>
                  <Badge variant="secondary" className="font-normal">
                    {r.tenant_name}
                  </Badge>
                </TableCell>
              ) : null}
              <TableCell>
                <span className="text-sm capitalize">{r.role}</span>
              </TableCell>
              <TableCell className="text-right">
                <Pencil
                  className="ml-auto size-3.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                  aria-hidden
                />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {footer}
    </>
  );
}

function TenantStatusCard({
  tenantName,
  policyCoverage,
  ruleCount,
  hasAuditEntries,
  loading,
}: {
  tenantName: string | null;
  policyCoverage: number | null;
  ruleCount: number;
  hasAuditEntries: boolean;
  loading?: boolean;
}) {
  return (
    <Card>
      <CardHeader className="border-b">
        <CardTitle className="flex items-center gap-2 text-base">
          <span className="relative inline-flex size-2.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-60" />
            <span className="relative inline-flex size-2.5 rounded-full bg-emerald-500" />
          </span>
          Tenant Status
        </CardTitle>
        <CardDescription>
          {tenantName ?? "All tenants — aggregate availability"}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5 p-5">
        <div className="grid gap-2">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Availability</span>
            <span className="font-mono text-foreground">
              {hasAuditEntries ? "Live" : "—"}
            </span>
          </div>
          <Progress value={hasAuditEntries ? 100 : 0} />
        </div>
        <div className="grid gap-2">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Policy Coverage</span>
            <span className="font-mono text-foreground">
              {loading ? "…" : policyCoverage === null ? "—" : `${policyCoverage}%`}
            </span>
          </div>
          <Progress value={policyCoverage ?? 0} />
          <p className="text-[11px] text-muted-foreground">
            {ruleCount === 0
              ? "No rules — default policy is in effect."
              : `${ruleCount} rule${ruleCount === 1 ? "" : "s"} active.`}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

function AuditLogCard({
  tenantId,
}: {
  tenantId?: string | null;
}) {
  const q = useMemo(
    () => (tenantId ? { tenant: tenantId, limit: 8 } : { limit: 8 }),
    [tenantId],
  );
  const decisions = useAuthzDecisions(q);
  const items = decisions.data?.items ?? [];
  return (
    <Card>
      <CardHeader className="border-b">
        <CardTitle className="flex items-center gap-2 text-base">
          <FileSearch className="size-4 text-muted-foreground" aria-hidden />
          Audit Log
        </CardTitle>
        <CardDescription>
          {tenantId ? "Recent decisions for this tenant" : "Recent decisions"}
        </CardDescription>
      </CardHeader>
      <CardContent className="p-0">
        {decisions.isLoading ? (
          <div className="grid gap-2 p-5">
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-3/4" />
            <Skeleton className="h-4 w-2/3" />
          </div>
        ) : decisions.error ? (
          <div className="p-5 text-xs text-destructive">
            {decisions.error.message}
          </div>
        ) : items.length === 0 ? (
          <div className="grid place-items-center gap-1 px-5 py-10 text-center">
            <Clock className="size-5 text-muted-foreground" aria-hidden />
            <p className="text-sm text-muted-foreground">No decisions yet.</p>
          </div>
        ) : (
          <ol className="relative grid gap-0 px-5 py-4">
            {items.map((d, i) => {
              const isAllow = d.effect === "allow";
              const Icon = isAllow ? CheckCircle2 : ShieldX;
              const iconCls = isAllow
                ? "text-emerald-600 dark:text-emerald-400"
                : "text-destructive";
              return (
                <li
                  key={`${d.at}:${i}`}
                  className="relative grid grid-cols-[1.25rem_1fr] gap-3 py-2"
                >
                  <div className="grid place-items-start pt-0.5">
                    <Icon className={cn("size-4", iconCls)} aria-hidden />
                  </div>
                  <div className="grid gap-0.5 text-xs">
                    <div className="flex items-baseline gap-2">
                      <span className="font-medium text-foreground">
                        {d.subject}
                      </span>
                      <span className="text-muted-foreground">{d.action}</span>
                      <code className="text-[10px] text-muted-foreground">
                        {d.kind}
                        {d.id ? `:${d.id}` : ""}
                      </code>
                    </div>
                    <span className="text-[10px] text-muted-foreground">
                      {new Date(d.at).toLocaleString()}
                      {d.reason ? ` · ${d.reason}` : ""}
                    </span>
                  </div>
                </li>
              );
            })}
          </ol>
        )}
      </CardContent>
    </Card>
  );
}

interface TenantAggregate {
  tenantId: string;
  rows: MemberRow[];
  teamCount: number;
  loading: boolean;
}

function TenantAggregateProbe({
  tenant,
  directory,
  onData,
}: {
  tenant: { id: string; display_name: string };
  directory: ReturnType<typeof useUserDirectory>;
  onData: (id: string, agg: TenantAggregate) => void;
}) {
  const members = useTenantMembers(tenant.id);
  const teams = useTeams(tenant.id);
  useEffect(() => {
    const rows: MemberRow[] = (members.data ?? []).map((m) => {
      const entry = directory?.getById?.(m.user_id);
      return {
        user_id: m.user_id,
        email: entry?.email ?? m.user_id,
        tenant_id: tenant.id,
        tenant_name: tenant.display_name,
        role: m.role,
      };
    });
    onData(tenant.id, {
      tenantId: tenant.id,
      rows,
      teamCount: teams.data?.length ?? 0,
      loading: members.isLoading || teams.isLoading,
    });
  }, [
    tenant.id,
    tenant.display_name,
    members.data,
    members.isLoading,
    teams.data,
    teams.isLoading,
    directory,
    onData,
  ]);
  return null;
}

function GlobalOverview({
  onSelect,
}: {
  onSelect: (n: SelectedNode) => void;
}) {
  const tenants = useTenants();
  const rules = useAuthzRules();
  const tenantList = tenants.data ?? [];
  const directory = useUserDirectory();

  const [aggMap, setAggMap] = useState<Record<string, TenantAggregate>>({});
  const handleAgg = useMemo(
    () => (id: string, agg: TenantAggregate) =>
      setAggMap((m) => (m[id] === agg ? m : { ...m, [id]: agg })),
    [],
  );

  const aggs = tenantList.map((t) => aggMap[t.id]).filter(Boolean) as TenantAggregate[];
  const rows = aggs.flatMap((a) => a.rows);
  const totalMembers = rows.length;
  const totalTeams = aggs.reduce((acc, a) => acc + a.teamCount, 0);
  const ruleCount = rules.data?.rules?.length ?? 0;
  const anyLoading =
    tenants.isLoading || aggs.length < tenantList.length || aggs.some((a) => a.loading);

  const topRows = rows.slice(0, 6);

  return (
    <div className="grid gap-6 lg:grid-cols-12">
      {tenantList.map((t) => (
        <TenantAggregateProbe
          key={t.id}
          tenant={t}
          directory={directory}
          onData={handleAgg}
        />
      ))}
      <div className="grid gap-6 lg:col-span-8">
        <div className="grid gap-4 sm:grid-cols-3">
          <StatCard
            label="Total Members"
            value={anyLoading ? "—" : totalMembers}
            icon={Users}
            loading={anyLoading}
            hint={`Across ${tenantList.length} tenant${tenantList.length === 1 ? "" : "s"}`}
          />
          <StatCard
            label="Active Teams"
            value={anyLoading ? "—" : totalTeams}
            icon={Layers}
            loading={anyLoading}
            tone="success"
            hint={totalTeams === 0 ? "No teams yet" : "Stable"}
          />
          <StatCard
            label="Active Rules"
            value={rules.isLoading ? "—" : ruleCount}
            icon={ShieldCheck}
            loading={rules.isLoading}
            tone={ruleCount === 0 ? "warning" : "default"}
            hint={
              ruleCount === 0
                ? "Default policy is in effect"
                : "Policy engine enforcing"
            }
          />
        </div>
        <Card className="overflow-hidden">
          <CardHeader className="flex flex-row items-center justify-between border-b">
            <div className="grid gap-0.5">
              <CardTitle className="text-base">Active Members</CardTitle>
              <CardDescription>
                Recent memberships across all tenants
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent className="p-0">
            <MembersTable
              rows={topRows}
              loading={anyLoading}
              showTenant
              emptyHint="No memberships yet — create a tenant and add a member."
              onSelectUser={(userId, tenantId) =>
                onSelect({ kind: "user", userId, tenantId })
              }
              footer={
                rows.length > topRows.length ? (
                  <div className="border-t px-5 py-3 text-center">
                    <button
                      type="button"
                      className="text-xs font-medium text-primary hover:underline"
                      onClick={() => onSelect({ kind: "root" })}
                    >
                      View all {rows.length} members
                    </button>
                  </div>
                ) : null
              }
            />
          </CardContent>
        </Card>
      </div>
      <aside className="grid gap-6 lg:col-span-4">
        <TenantStatusCard
          tenantName={null}
          policyCoverage={ruleCount === 0 ? 0 : Math.min(100, ruleCount * 5)}
          ruleCount={ruleCount}
          hasAuditEntries
          loading={rules.isLoading}
        />
        <AuditLogCard />
      </aside>
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
        <TenantMembersTab tenantId={tenantId} />
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

function TenantMembersTab({ tenantId }: { tenantId: string }) {
  const members = useTenantMembers(tenantId);
  return (
    <MembersPanel
      tenantId={tenantId}
      members={members.data}
      membersLoading={members.isLoading}
      membersError={members.error}
    />
  );
}

function TenantOverview({ tenantId }: { tenantId: string }) {
  const tenants = useTenants();
  const members = useTenantMembers(tenantId);
  const teams = useTeams(tenantId);
  const rules = useAuthzRules();
  const directory = useUserDirectory();
  const t = (tenants.data ?? []).find((x) => x.id === tenantId);

  const rows: MemberRow[] = useMemo(() => {
    if (!t) return [];
    return (members.data ?? []).map((m) => {
      const entry = directory?.getById?.(m.user_id);
      return {
        user_id: m.user_id,
        email: entry?.email ?? m.user_id,
        tenant_id: t.id,
        tenant_name: t.display_name,
        role: m.role,
      };
    });
  }, [members.data, directory, t]);

  if (!t) return <StateRow variant="empty">Tenant not found.</StateRow>;

  const memberCount = rows.length;
  const teamCount = teams.data?.length ?? 0;
  const tenantRules =
    rules.data?.rules?.filter((r) => !r.tenant_id || r.tenant_id === t.id) ?? [];
  const ruleCount = tenantRules.length;
  const loading = members.isLoading || teams.isLoading;
  const topRows = rows.slice(0, 6);

  return (
    <div className="grid gap-6 lg:grid-cols-12">
      <div className="grid gap-6 lg:col-span-8">
        <div className="grid gap-4 sm:grid-cols-3">
          <StatCard
            label="Total Members"
            value={loading ? "—" : memberCount}
            icon={Users}
            loading={loading}
            hint={t.display_name}
          />
          <StatCard
            label="Active Teams"
            value={loading ? "—" : teamCount}
            icon={Layers}
            loading={loading}
            tone="success"
            hint={teamCount === 0 ? "No teams yet" : "Stable"}
          />
          <StatCard
            label="Active Rules"
            value={rules.isLoading ? "—" : ruleCount}
            icon={ShieldCheck}
            loading={rules.isLoading}
            tone={ruleCount === 0 ? "warning" : "default"}
            hint={
              ruleCount === 0
                ? "No rules — default policy applies"
                : "Tenant + global rules"
            }
          />
        </div>
        <Card className="overflow-hidden">
          <CardHeader className="border-b">
            <div className="grid gap-0.5">
              <CardTitle className="text-base">Active Members</CardTitle>
              <CardDescription>
                Members of {t.display_name}
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent className="p-0">
            <MembersTable
              rows={topRows}
              loading={loading}
              emptyHint="No members yet — add one from the Members tab."
              footer={
                rows.length > topRows.length ? (
                  <div className="border-t px-5 py-3 text-center">
                    <span className="text-xs text-muted-foreground">
                      Showing {topRows.length} of {rows.length} members
                    </span>
                  </div>
                ) : null
              }
            />
          </CardContent>
        </Card>
      </div>
      <aside className="grid gap-6 lg:col-span-4">
        <TenantStatusCard
          tenantName={t.display_name}
          policyCoverage={ruleCount === 0 ? 0 : Math.min(100, ruleCount * 10)}
          ruleCount={ruleCount}
          hasAuditEntries
          loading={rules.isLoading}
        />
        <AuditLogCard tenantId={t.id} />
      </aside>
    </div>
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
