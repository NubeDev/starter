// Master-detail Access Control surface — shared by the
// `/admin/access` index route and the `/admin/access/*` catch-all.
// URL <-> selection mapping lives here so deep links such as
// `/admin/access/t/acme/team/platform` round-trip cleanly.

import { useMemo } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import {
  AuthzAdmin,
  type AuthzMessages,
  type SelectedNode,
  type UserDirectory,
  type UserDirectoryEntry,
  type UsersAdminOps,
} from '@nube/starter-ui-authz'
import { useTenants } from '@nube/starter-ui-authz'
import {
  useUserList,
  useUserCreate,
  useUserDisable,
  useUndoLast,
} from '@nube/rubix-client-react'

// --------------------------------------------------------------------------
// i18n adapter
// --------------------------------------------------------------------------

export function useAuthzMessagesFromIntl(): Partial<AuthzMessages> {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  return {
    shell: {
      title: tr('access.shell.title', 'Access Control'),
      tabs: {
        tenants: tr('access.tabs.tenants', 'Tenants'),
        teams: tr('access.tabs.teams', 'Teams'),
        members: tr('access.tabs.members', 'Members'),
        rules: tr('access.tabs.rules', 'Rules'),
        assignments: tr('access.tabs.assignments', 'Assignments'),
        resources: tr('access.tabs.resources', 'Resources'),
        check: tr('access.tabs.check', 'Check'),
        decisions: tr('access.tabs.decisions', 'Decisions'),
      },
    },
    common: {
      loading: tr('access.common.loading', 'Loading…'),
      empty: tr('access.common.empty', 'Nothing here yet.'),
      error: tr('access.common.error', 'Something went wrong.'),
      save: tr('access.common.save', 'Save'),
      create: tr('access.common.create', 'Create'),
      cancel: tr('access.common.cancel', 'Cancel'),
      delete: tr('access.common.delete', 'Delete'),
      confirmDelete: tr('access.common.confirmDelete', 'Are you sure?'),
      edit: tr('access.common.edit', 'Edit'),
      refresh: tr('access.common.refresh', 'Refresh'),
      subject: tr('access.common.subject', 'Subject'),
      role: tr('access.common.role', 'Role'),
      action: tr('access.common.action', 'Action'),
      resource: tr('access.common.resource', 'Resource'),
      effect: tr('access.common.effect', 'Effect'),
      priority: tr('access.common.priority', 'Priority'),
      tenant: tr('access.common.tenant', 'Tenant'),
      createdBy: tr('access.common.createdBy', 'Created by'),
      at: tr('access.common.at', 'When'),
      reason: tr('access.common.reason', 'Reason'),
      allow: tr('access.common.allow', 'Allow'),
      deny: tr('access.common.deny', 'Deny'),
      any: tr('access.common.any', 'Any'),
    },
    tenants: {
      title: tr('access.tenants.title', 'Tenants'),
      description: tr('access.tenants.description', 'Organisational scopes.'),
      columns: {
        slug: tr('access.tenants.columns.slug', 'Slug'),
        displayName: tr('access.tenants.columns.displayName', 'Name'),
        auditSample: tr('access.tenants.columns.auditSample', 'Audit sample'),
      },
      form: {
        slugLabel: tr('access.tenants.form.slugLabel', 'Slug'),
        slugPlaceholder: tr('access.tenants.form.slugPlaceholder', 'acme'),
        displayNameLabel: tr('access.tenants.form.displayNameLabel', 'Display name'),
        displayNamePlaceholder: tr('access.tenants.form.displayNamePlaceholder', 'Acme Corp'),
        submit: tr('access.tenants.form.submit', 'Create tenant'),
      },
    },
    members: {
      title: tr('access.members.title', 'Members'),
      description: tr('access.members.description', 'A member is a user with a role in this tenant.'),
      selectTenantPrompt: tr('access.members.selectTenantPrompt', 'Select a tenant to manage its members.'),
      columns: { user: tr('access.members.columns.user', 'User') },
      form: {
        userIdLabel: tr('access.members.form.userIdLabel', 'User id'),
        userIdPlaceholder: tr('access.members.form.userIdPlaceholder', 'user-id or email'),
        roleLabel: tr('access.members.form.roleLabel', 'Role'),
        submit: tr('access.members.form.submit', 'Add member'),
      },
    },
    teams: {
      title: tr('access.teams.title', 'Teams'),
      description: tr('access.teams.description', 'Named groups inside a tenant.'),
      selectTenantPrompt: tr('access.teams.selectTenantPrompt', 'Select a tenant to manage its teams.'),
      columns: {
        slug: tr('access.teams.columns.slug', 'Slug'),
        displayName: tr('access.teams.columns.displayName', 'Name'),
        members: tr('access.teams.columns.members', 'Members'),
      },
      form: {
        slugLabel: tr('access.teams.form.slugLabel', 'Slug'),
        displayNameLabel: tr('access.teams.form.displayNameLabel', 'Display name'),
        submit: tr('access.teams.form.submit', 'Create team'),
      },
      teamMembers: {
        title: tr('access.teams.teamMembers.title', 'Team members'),
        userIdLabel: tr('access.teams.teamMembers.userIdLabel', 'User id'),
        add: tr('access.teams.teamMembers.add', 'Add'),
      },
    },
    rules: {
      title: tr('access.rules.title', 'Rules'),
      description: tr('access.rules.description', 'Per-role and per-resource policy.'),
      form: {
        roleLabel: tr('access.rules.form.roleLabel', 'Role'),
        resourceLabel: tr('access.rules.form.resourceLabel', 'Resource'),
        actionsLabel: tr('access.rules.form.actionsLabel', 'Actions'),
        actionsPlaceholder: tr('access.rules.form.actionsPlaceholder', 'read, write, *'),
        conditionLabel: tr('access.rules.form.conditionLabel', 'Condition'),
        conditionPlaceholder: tr('access.rules.form.conditionPlaceholder', 'owner'),
        effectLabel: tr('access.rules.form.effectLabel', 'Effect'),
        priorityLabel: tr('access.rules.form.priorityLabel', 'Priority'),
        tenantLabel: tr('access.rules.form.tenantLabel', 'Tenant'),
        tenantPlaceholderGlobal: tr('access.rules.form.tenantPlaceholderGlobal', '(global)'),
        submit: tr('access.rules.form.submit', 'Create rule'),
      },
    },
    assignments: {
      title: tr('access.assignments.title', 'Assignments'),
      description: tr('access.assignments.description', 'Bind a subject to a role.'),
      form: {
        subjectLabel: tr('access.assignments.form.subjectLabel', 'Subject'),
        subjectPlaceholder: tr('access.assignments.form.subjectPlaceholder', 'user-id or user-*'),
        roleLabel: tr('access.assignments.form.roleLabel', 'Role'),
        submit: tr('access.assignments.form.submit', 'Create assignment'),
      },
    },
    resources: {
      title: tr('access.resources.title', 'Resources'),
      description: tr('access.resources.description', 'Every resource kind the engine knows.'),
      columns: {
        kind: tr('access.resources.columns.kind', 'Kind'),
        label: tr('access.resources.columns.label', 'Label'),
        actions: tr('access.resources.columns.actions', 'Actions'),
        ownership: tr('access.resources.columns.ownership', 'Ownership'),
        tenantScoped: tr('access.resources.columns.tenantScoped', 'Tenant-scoped'),
      },
    },
    check: {
      title: tr('access.check.title', 'Dry-run check'),
      description: tr('access.check.description', 'Preview what the engine would decide right now.'),
      principalSubjectLabel: tr('access.check.principalSubjectLabel', 'Principal subject'),
      principalRoleLabel: tr('access.check.principalRoleLabel', 'Principal role'),
      actionLabel: tr('access.check.actionLabel', 'Action'),
      resourceKindLabel: tr('access.check.resourceKindLabel', 'Resource kind'),
      resourceIdLabel: tr('access.check.resourceIdLabel', 'Resource id'),
      resourceOwnerLabel: tr('access.check.resourceOwnerLabel', 'Resource owner'),
      submit: tr('access.check.submit', 'Check'),
      decisionAllow: tr('access.check.decisionAllow', 'Allow'),
      decisionDeny: tr('access.check.decisionDeny', 'Deny'),
      matchedRule: tr('access.check.matchedRule', 'Matched rule'),
    },
    decisions: {
      title: tr('access.decisions.title', 'Decisions'),
      description: tr('access.decisions.description', 'Paged read of the engine audit sink. Newest first.'),
      filters: {
        tenantLabel: tr('access.decisions.filters.tenantLabel', 'Tenant'),
        subjectLabel: tr('access.decisions.filters.subjectLabel', 'Subject'),
        effectLabel: tr('access.decisions.filters.effectLabel', 'Effect'),
        apply: tr('access.decisions.filters.apply', 'Apply'),
        reset: tr('access.decisions.filters.reset', 'Reset'),
      },
      loadMore: tr('access.decisions.loadMore', 'Load more'),
      endOfList: tr('access.decisions.endOfList', 'End of list.'),
    },
  }
}

// --------------------------------------------------------------------------
// Adapters into `@nube/starter-ui-authz`
// --------------------------------------------------------------------------

function useRubixUserDirectory(): UserDirectory {
  // `useUserList()` loads the full roster once; the picker filters in
  // memory. Swap for a server-side `?q=` search when the API grows one.
  const list = useUserList()
  const users: UserDirectoryEntry[] = useMemo(
    () =>
      (list.data?.users ?? []).map((u) => ({
        user_id: u.user_id,
        email: u.email,
        role: u.role,
        disabled_at_ms: u.disabled_at_ms,
      })),
    [list.data],
  )
  return useMemo<UserDirectory>(
    () => ({
      search(query) {
        const q = query.trim().toLowerCase()
        if (!q) return users.slice(0, 20)
        return users
          .filter(
            (u) =>
              u.email.toLowerCase().includes(q) ||
              u.user_id.toLowerCase().includes(q),
          )
          .slice(0, 20)
      },
      getById(id) {
        return users.find((u) => u.user_id === id)
      },
    }),
    [users],
  )
}

function useRubixUserOps(): UsersAdminOps {
  const list = useUserList()
  const create = useUserCreate()
  const disable = useUserDisable()
  const undo = useUndoLast()

  return useMemo<UsersAdminOps>(() => {
    const users = list.data?.users ?? []
    return {
      list: () => (list.data ? { users: list.data.users } : undefined),
      get: (id) => users.find((u) => u.user_id === id),
      create: async ({ email, role }) => {
        await create.mutateAsync({ email, role })
      },
      disable: async (userId) => {
        await disable.mutateAsync({ user_id: userId })
      },
      undoLast: async () => {
        await undo.mutateAsync({})
      },
      isCreating: create.isPending,
      isDisabling: disable.isPending,
      isUndoing: undo.isPending,
    }
  }, [list.data, create, disable, undo])
}

// --------------------------------------------------------------------------
// URL <-> SelectedNode mapping
// --------------------------------------------------------------------------

// URL shape:
//   ""                            -> root
//   "t/<slug>"                    -> tenant
//   "t/<slug>/team/<slug>"        -> team
//   "t/<slug>/u/<userId>"         -> user (in tenant)
//   "u/<userId>"                  -> user (global)
//
// Slugs in the URL — resolved to ids via `useTenants()` cache. When a
// slug cannot be resolved (cache empty, tenant deleted) we degrade to
// the root view rather than throwing.

function splatToSelected(
  splat: string,
  tenants: { id: string; slug: string }[],
  teamsByTenant: Map<string, { id: string; slug: string }[]>,
): SelectedNode {
  const segs = (splat || '').split('/').filter(Boolean)
  if (segs.length === 0) return { kind: 'root' }
  if (segs[0] === 'u' && segs[1]) return { kind: 'user', userId: segs[1] }
  if (segs[0] === 't' && segs[1]) {
    const tenant = tenants.find((t) => t.slug === segs[1])
    if (!tenant) return { kind: 'root' }
    if (segs[2] === 'team' && segs[3]) {
      const teams = teamsByTenant.get(tenant.id) ?? []
      const team = teams.find((tm) => tm.slug === segs[3])
      if (!team) return { kind: 'tenant', tenantId: tenant.id }
      return { kind: 'team', tenantId: tenant.id, teamId: team.id }
    }
    if (segs[2] === 'u' && segs[3])
      return { kind: 'user', userId: segs[3], tenantId: tenant.id }
    return { kind: 'tenant', tenantId: tenant.id }
  }
  return { kind: 'root' }
}

function selectedToPath(
  sel: SelectedNode,
  tenants: { id: string; slug: string }[],
  teamsByTenant: Map<string, { id: string; slug: string }[]>,
): string {
  if (sel.kind === 'root') return '/admin/access'
  if (sel.kind === 'user' && !sel.tenantId)
    return `/admin/access/u/${sel.userId}`
  const tenantId: string =
    sel.kind === 'user' ? (sel.tenantId ?? '') : sel.tenantId
  const tenant = tenants.find((t) => t.id === tenantId)
  const tenantSlug = tenant?.slug ?? tenantId
  if (sel.kind === 'tenant') return `/admin/access/t/${tenantSlug}`
  if (sel.kind === 'user')
    return `/admin/access/t/${tenantSlug}/u/${sel.userId}`
  // team
  const teams = teamsByTenant.get(tenantId) ?? []
  const team = teams.find((tm) => tm.id === sel.teamId)
  const teamSlug = team?.slug ?? sel.teamId
  return `/admin/access/t/${tenantSlug}/team/${teamSlug}`
}

// --------------------------------------------------------------------------
// Component
// --------------------------------------------------------------------------

export interface AccessControlProps {
  /** Trailing path under `/admin/access` — e.g. `"t/acme/team/platform"`. */
  splat: string
}

export function AccessControl({ splat }: AccessControlProps) {
  const i18n = useAuthzMessagesFromIntl()
  const userDirectory = useRubixUserDirectory()
  const userOps = useRubixUserOps()
  const navigate = useNavigate()

  // `useTenants` is the cache the rail uses too; reading it here is
  // free (React Query dedupes the request).
  const tenants = useTenants()
  const tenantList = useMemo(
    () => (tenants.data ?? []).map((t) => ({ id: t.id, slug: t.slug })),
    [tenants.data],
  )
  // Teams are fetched per-tenant; we lazily resolve only the slug
  // currently in the URL via a small helper component below. For
  // round-tripping path -> selection we only need the *current* tenant.
  const teamsByTenant = useMemo(() => new Map<string, { id: string; slug: string }[]>(), [])

  const selected = useMemo(
    () => splatToSelected(splat, tenantList, teamsByTenant),
    [splat, tenantList, teamsByTenant],
  )

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            Admin / Access
            {selected.kind === 'tenant' ||
            selected.kind === 'team' ||
            (selected.kind === 'user' && selected.tenantId)
              ? (() => {
                  const tid =
                    selected.kind === 'user'
                      ? selected.tenantId!
                      : selected.tenantId
                  const tn = tenantList.find((x) => x.id === tid)?.slug
                  return tn ? ` / ${tn}` : ''
                })()
              : ''}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {i18n.shell?.title}
        </h1>
        <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
          Manage tenants, teams, members, and policy from one place.
        </p>
      </header>
      <AuthzAdmin
        i18n={i18n}
        header={<></>}
        userDirectory={userDirectory}
        userOps={userOps}
        selectedNode={selected}
        onSelectNode={(next) => {
          const to = selectedToPath(next, tenantList, teamsByTenant)
          void navigate({ to })
        }}
      />
    </section>
  )
}
