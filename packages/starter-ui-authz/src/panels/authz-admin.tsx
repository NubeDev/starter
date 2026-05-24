// `<AuthzAdmin>` — tabbed shell composing every panel. Carries
// the selected-tenant state across panels (so picking a tenant in
// the Tenants tab scopes the Members and Teams tabs to it). The
// shell itself is opinionated; callers wanting a different layout
// can mount the panels directly from `./panels`.

import { useState, type ReactNode } from "react";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit";
import { AuthzI18nProvider } from "../i18n/context.js";
import type { AuthzMessages } from "../i18n/messages.js";
import { useAuthzMessages } from "../i18n/context.js";
import { TenantsPanel } from "./tenants-panel.js";
import { MembersPanel } from "./members-panel.js";
import { TeamsPanel } from "./teams-panel.js";
import { RulesPanel } from "./rules-panel.js";
import { AssignmentsPanel } from "./assignments-panel.js";
import { ResourcesPanel } from "./resources-panel.js";
import { CheckPanel } from "./check-panel.js";
import { DecisionsPanel } from "./decisions-panel.js";

export type AuthzAdminTab =
  | "tenants"
  | "teams"
  | "members"
  | "rules"
  | "assignments"
  | "resources"
  | "check"
  | "decisions";

export interface AuthzAdminProps {
  /** Initial tab. Defaults to `"tenants"`. */
  defaultTab?: AuthzAdminTab;
  /** Optional i18n override merged on top of `DEFAULT_AUTHZ_MESSAGES`. */
  i18n?: Partial<AuthzMessages>;
  /** Initial selected tenant id. Defaults to none. */
  initialTenantId?: string;
  /** Optional slot rendered above the tabs (page header). */
  header?: ReactNode;
}

export function AuthzAdmin({ defaultTab = "tenants", i18n, initialTenantId, header }: AuthzAdminProps) {
  return (
    <AuthzI18nProvider value={i18n}>
      <AuthzAdminInner defaultTab={defaultTab} initialTenantId={initialTenantId ?? null} header={header} />
    </AuthzI18nProvider>
  );
}

function AuthzAdminInner({
  defaultTab,
  initialTenantId,
  header,
}: {
  defaultTab: AuthzAdminTab;
  initialTenantId: string | null;
  header?: ReactNode;
}) {
  const m = useAuthzMessages();
  const [tenantId, setTenantId] = useState<string | null>(initialTenantId);
  const [tab, setTab] = useState<AuthzAdminTab>(defaultTab);

  return (
    <div className="grid gap-6">
      {header ?? (
        <header>
          <h1 className="text-2xl font-semibold tracking-tight">{m.shell.title}</h1>
        </header>
      )}

      <Tabs value={tab} onValueChange={(v) => setTab(v as AuthzAdminTab)}>
        <TabsList className="flex-wrap">
          <TabsTrigger value="tenants">{m.shell.tabs.tenants}</TabsTrigger>
          <TabsTrigger value="members">{m.shell.tabs.members}</TabsTrigger>
          <TabsTrigger value="teams">{m.shell.tabs.teams}</TabsTrigger>
          <TabsTrigger value="rules">{m.shell.tabs.rules}</TabsTrigger>
          <TabsTrigger value="assignments">{m.shell.tabs.assignments}</TabsTrigger>
          <TabsTrigger value="resources">{m.shell.tabs.resources}</TabsTrigger>
          <TabsTrigger value="check">{m.shell.tabs.check}</TabsTrigger>
          <TabsTrigger value="decisions">{m.shell.tabs.decisions}</TabsTrigger>
        </TabsList>

        <TabsContent value="tenants" className="mt-6">
          <TenantsPanel
            selectedTenantId={tenantId}
            onSelectTenant={(id) => {
              setTenantId(id);
            }}
          />
        </TabsContent>
        <TabsContent value="members" className="mt-6">
          <MembersPanel tenantId={tenantId} />
        </TabsContent>
        <TabsContent value="teams" className="mt-6">
          <TeamsPanel tenantId={tenantId} />
        </TabsContent>
        <TabsContent value="rules" className="mt-6">
          <RulesPanel />
        </TabsContent>
        <TabsContent value="assignments" className="mt-6">
          <AssignmentsPanel />
        </TabsContent>
        <TabsContent value="resources" className="mt-6">
          <ResourcesPanel />
        </TabsContent>
        <TabsContent value="check" className="mt-6">
          <CheckPanel />
        </TabsContent>
        <TabsContent value="decisions" className="mt-6">
          <DecisionsPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
