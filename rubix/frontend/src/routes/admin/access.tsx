// `/admin/access` — tenants, teams, members, authz rules,
// assignments, the resource registry, the dry-run check tool, and
// the decisions audit feed. Backed by `@nube/starter-ui-authz`
// which speaks to the starter backend via `StarterClient`
// (`StarterClientProvider` is mounted at app root in `main.tsx`).

import { createFileRoute } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { AuthzAdmin, type AuthzMessages } from '@nube/starter-ui-authz'
import { ErrorBoundary } from '@/components/error-boundary'

function useAuthzMessagesFromIntl(): Partial<AuthzMessages> {
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
      description: tr('access.members.description', 'Users with a role binding in this tenant.'),
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

function AccessPanel() {
  const i18n = useAuthzMessagesFromIntl()
  return (
    <section className="relative mx-auto max-w-6xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            Admin
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {i18n.shell?.title}
        </h1>
      </header>
      <AuthzAdmin i18n={i18n} header={<></>} />
    </section>
  )
}

function AccessRoute() {
  return (
    <ErrorBoundary>
      <AccessPanel />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/access')({ component: AccessRoute })
