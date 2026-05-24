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
