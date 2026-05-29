// Per-extension page route.
//
// Matches `/extensions/:extId/*rest`. Two surfaces stack here:
//
//  1. The host **admin panel** (`ExtensionAdminPanel`) — a tabbed
//     management chrome the rubix host owns: Issues (severity-coloured
//     diagnostics), Process (pid / uptime / mem / cpu), Metrics
//     (counters), plus an Uninstall dialog that previews the cleanup
//     manifest before purging. It reads the comprehensive
//     `/extensions/{id}/{issues,process,metrics,cleanup}` endpoints via
//     the `useExtension*` hooks.
//  2. The extension's own federated UI via `<ExtensionSlot/>`, which
//     mounts the `main` slot and forwards the splat as `route` so the
//     extension's `Main` component can dispatch its own sub-pages.
//
// Extensions are auto-loaded at boot by `ExtensionAutoLoader`
// (see `src/lib/extension-autoloader.tsx`), so by the time the
// user clicks an extension nav link the slot resolver already has
// a `Main` component ready to mount.

import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  AlertTriangle,
  Cpu,
  Gauge,
  ListChecks,
  Loader2,
  Trash2,
} from 'lucide-react'
import { Button } from '@nube/starter-ui-kit'
import { ExtensionSlot } from '@nube/starter-ext-ui'
import {
  useExtensionIssues,
  useExtensionProcess,
  useExtensionMetrics,
  type ExtensionIssue,
} from '@nube/rubix-client-react'
import {
  UninstallDialog,
  formatBytes,
  formatUptime,
} from '@/components/extensions/uninstall-dialog'

export const Route = createFileRoute('/extensions/$extId/$')({
  component: RouteComponent,
})

function RouteComponent() {
  const { extId, _splat } = Route.useParams()
  const splat = _splat ?? ''
  // `/extensions/<id>/admin` shows the host admin chrome (Issues/Process/
  // Metrics/Uninstall). Any other splat — including the empty root —
  // hands the surface entirely to the extension's own federated UI so it
  // can dispatch its own sub-pages without the host getting in the way.
  const isAdmin = splat === 'admin' || splat.startsWith('admin/')
  return (
    <div className="mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      {isAdmin ? (
        <ExtensionAdminPanel extId={extId} />
      ) : (
        <ExtensionSlot id="main" extensionId={extId} route={splat} />
      )}
    </div>
  )
}

type AdminTab = 'issues' | 'process' | 'metrics'

// ---------------------------------------------------------------------------
// Admin panel — tabbed host chrome around the extension's own UI.
// ---------------------------------------------------------------------------

function ExtensionAdminPanel({ extId }: { extId: string }) {
  const intl = useIntl()
  const tr = (id: string, def: string) => intl.formatMessage({ id, defaultMessage: def })
  const navigate = useNavigate()
  const [tab, setTab] = useState<AdminTab>('issues')
  const [uninstallOpen, setUninstallOpen] = useState(false)

  const tabs: Array<{ key: AdminTab; label: string; icon: typeof ListChecks }> = [
    { key: 'issues', label: tr('extensions.tab.issues', 'Issues'), icon: ListChecks },
    { key: 'process', label: tr('extensions.tab.process', 'Process'), icon: Cpu },
    { key: 'metrics', label: tr('extensions.tab.metrics', 'Metrics'), icon: Gauge },
  ]

  return (
    <section className="glass mb-6 overflow-hidden rounded-3xl">
      <header className="flex items-center justify-between gap-4 border-b border-[color:var(--color-border)] px-6 py-4">
        <div className="flex items-center gap-1">
          {tabs.map((t) => {
            const Icon = t.icon
            const active = tab === t.key
            return (
              <button
                key={t.key}
                type="button"
                onClick={() => setTab(t.key)}
                className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium transition ${
                  active
                    ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)] ring-1 ring-[color:var(--color-leaf)]/30'
                    : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]'
                }`}
              >
                <Icon className="h-3.5 w-3.5" />
                {t.label}
              </button>
            )
          })}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() => setUninstallOpen(true)}
          title={tr('extensions.uninstall.title', 'Uninstall extension')}
        >
          <Trash2 className="h-3.5 w-3.5" />
          {tr('extensions.uninstall.action', 'Uninstall')}
        </Button>
      </header>

      <div className="px-6 py-5">
        {tab === 'issues' ? <IssuesTab extId={extId} /> : null}
        {tab === 'process' ? <ProcessTab extId={extId} /> : null}
        {tab === 'metrics' ? <MetricsTab extId={extId} /> : null}
      </div>

      {uninstallOpen ? (
        <UninstallDialog
          extId={extId}
          onClose={() => setUninstallOpen(false)}
          onUninstalled={() => void navigate({ to: '/extensions' })}
        />
      ) : null}
    </section>
  )
}

// ---------------------------------------------------------------------------
// Issues
// ---------------------------------------------------------------------------

function severityClasses(severity: string): string {
  switch (severity) {
    case 'fatal':
    case 'error':
      return 'bg-red-500/10 text-red-400 ring-red-500/30'
    case 'warning':
      return 'bg-[color:var(--color-sun)]/15 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/30'
    default:
      return 'bg-[color:var(--color-aqua)]/15 text-[color:var(--color-aqua)] ring-[color:var(--color-aqua)]/30'
  }
}

/** Map a stable `ext.issue.*` code onto its rubix MessageKey. */
function issueMessageKey(code: string): string {
  // `ext.issue.crashed` → `rubix.extension.issue.crashed`.
  const suffix = code.startsWith('ext.issue.') ? code.slice('ext.issue.'.length) : code
  return `rubix.extension.issue.${suffix}`
}

function IssuesTab({ extId }: { extId: string }) {
  const intl = useIntl()
  const tr = (id: string, def: string) => intl.formatMessage({ id, defaultMessage: def })
  const q = useExtensionIssues(extId)

  if (q.isLoading) return <Loading />
  if (q.isError) return <ErrorRow message={q.error.message} />
  const issues = q.data?.issues ?? []
  if (issues.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-subtle)]">
        {tr('extensions.issues.empty', 'No issues reported.')}
      </p>
    )
  }
  return (
    <ul className="space-y-2">
      {issues.map((issue: ExtensionIssue, i: number) => (
        <li
          key={`${issue.code}-${issue.seq ?? i}`}
          className="flex items-start gap-3 rounded-2xl border border-[color:var(--color-border)]/60 px-4 py-3"
        >
          <span
            className={`mt-0.5 inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider ring-1 ${severityClasses(issue.severity)}`}
          >
            <AlertTriangle className="h-3 w-3" />
            {issue.severity}
          </span>
          <div className="min-w-0">
            <div className="text-sm font-medium text-[color:var(--color-text)]">
              {tr(issueMessageKey(issue.code), issue.code)}
            </div>
            <div className="break-words text-xs text-[color:var(--color-muted)]">{issue.detail}</div>
            <div className="mt-0.5 font-mono text-[10px] text-[color:var(--color-subtle)]">
              {issue.source}
              {typeof issue.seq === 'number' ? ` · seq ${issue.seq}` : ''}
            </div>
          </div>
        </li>
      ))}
    </ul>
  )
}

// ---------------------------------------------------------------------------
// Process
// ---------------------------------------------------------------------------

function ProcessTab({ extId }: { extId: string }) {
  const intl = useIntl()
  const tr = (id: string, def: string) => intl.formatMessage({ id, defaultMessage: def })
  const q = useExtensionProcess(extId)

  if (q.isLoading) return <Loading />
  // A `404 ext.process.not_running` is expected for builtin/wasm/stopped —
  // we hide the stats rather than show an error.
  if (q.isError || !q.data) {
    return (
      <p className="text-sm text-[color:var(--color-subtle)]">
        {tr('extensions.process.notRunning', 'No live process — this extension is builtin, WASM, or stopped.')}
      </p>
    )
  }
  const p = q.data
  const stats: Array<{ label: string; value: string }> = [
    { label: tr('extensions.process.pid', 'PID'), value: String(p.pid) },
    { label: tr('extensions.process.uptime', 'Uptime'), value: formatUptime(p.uptime) },
    { label: tr('extensions.process.mem', 'Memory (RSS)'), value: formatBytes(p.rss_bytes) },
    {
      label: tr('extensions.process.cpu', 'CPU'),
      value: p.cpu_pct == null ? '—' : `${p.cpu_pct.toFixed(1)}%`,
    },
    { label: tr('extensions.process.restarts', 'Restarts'), value: String(p.restarts) },
  ]
  return <StatGrid stats={stats} />
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

function MetricsTab({ extId }: { extId: string }) {
  const intl = useIntl()
  const tr = (id: string, def: string) => intl.formatMessage({ id, defaultMessage: def })
  const q = useExtensionMetrics(extId)

  if (q.isLoading) return <Loading />
  if (q.isError) return <ErrorRow message={q.error.message} />
  const m = q.data
  if (!m) return null
  const stats: Array<{ label: string; value: string }> = [
    { label: tr('extensions.metrics.state', 'Lifecycle'), value: m.lifecycle_state },
    { label: tr('extensions.metrics.toolCalls', 'Tool calls'), value: String(m.tool_calls_total) },
    { label: tr('extensions.metrics.toolErrors', 'Tool errors'), value: String(m.tool_errors_total) },
    { label: tr('extensions.metrics.restRequests', 'REST requests'), value: String(m.rest_requests_total) },
    { label: tr('extensions.metrics.workerRuns', 'Worker runs'), value: String(m.worker_runs_total) },
    { label: tr('extensions.metrics.workerFailures', 'Worker failures'), value: String(m.worker_failures_total) },
    { label: tr('extensions.metrics.restarts', 'Restarts'), value: String(m.restarts_total) },
    {
      label: tr('extensions.metrics.capViolations', 'Capability violations'),
      value: String(m.capability_violations_total),
    },
    { label: tr('extensions.metrics.eventsDropped', 'Events dropped'), value: String(m.events_dropped_total) },
  ]
  return <StatGrid stats={stats} />
}

// ---------------------------------------------------------------------------
// Small shared bits
// ---------------------------------------------------------------------------

function StatGrid({ stats }: { stats: Array<{ label: string; value: string }> }) {
  return (
    <dl className="grid grid-cols-2 gap-4 sm:grid-cols-3">
      {stats.map((s) => (
        <div key={s.label} className="rounded-2xl border border-[color:var(--color-border)]/60 px-4 py-3">
          <dt className="text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">{s.label}</dt>
          <dd className="mt-1 font-mono text-lg text-[color:var(--color-text)]">{s.value}</dd>
        </div>
      ))}
    </dl>
  )
}

function Loading() {
  return (
    <div className="flex items-center gap-2 text-sm text-[color:var(--color-subtle)]">
      <Loader2 className="h-4 w-4 animate-spin" />
      Loading…
    </div>
  )
}

function ErrorRow({ message }: { message: string }) {
  return <p className="text-sm text-red-400">{message}</p>
}
