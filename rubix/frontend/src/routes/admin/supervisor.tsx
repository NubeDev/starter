// `/admin/supervisor` — Supervisor Health.
//
// Surfaces the extension supervisor's runtime stats with emphasis on
// the orphaned-child-process leak the backend now guards against:
//
//   - A boot-time "reaper" summary: process groups left alive by a
//     previously SIGKILL'd agent instance, reclaimed at this startup.
//   - A per-extension table whose marquee column is `group_kills_total`
//     — the number of times the supervisor had to SIGKILL an extension's
//     whole process group because the child (or a grandchild) leaked.
//     A rising value flags a process-leaking extension.
//
// Data sources (both via `client.starter.fetch`, so CSRF + auth cookies
// ride along automatically). These are two SEPARATE, independent calls:
//   - TABLE        `GET /api/v1/extensions/overview`      → [ ...rows ]
//     Always drives the per-extension table (incl. `group_kills_total`).
//   - REAPER CARD  `GET /api/v1/admin/supervisor/health`  → { reaped }
//     Drives only the boot-reaper card. `reaped` carries just `groups`;
//     `total`/`killed` are NOT serialized — they're derived in the UI.
//
// The health endpoint is treated defensively: if it 404s (or otherwise
// errors) we show a "telemetry unavailable" note on the reaper card but
// STILL render the table from /overview.

import { createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { useIntl } from 'react-intl'
import { useQuery } from '@tanstack/react-query'
import { Activity, HeartPulse, ShieldAlert, ShieldCheck } from 'lucide-react'
import { Skeleton } from '@nube/starter-ui-kit'
import { useRubixClient } from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'
import {
  formatBytes,
  formatUptime,
} from '@/components/extensions/uninstall-dialog'

// ---------- types mirroring the supervisor wire payloads -----------

interface ProcessStats {
  pid?: number | null
  started_at?: string | null
  uptime?: { secs: number; nanos?: number } | null
  rss_bytes?: number | null
  cpu_pct?: number | null
  restarts?: number | null
}

/** One row per extension. Superset of `ExtensionOverviewRow` — adds the
 * NEW `group_kills_total` field (not yet in the shared client type). */
interface SupervisorExtensionRow {
  id: string
  version?: string | null
  display_name?: string | null
  runtime_kind?: string | null
  enabled?: 'enabled' | 'disabled'
  restart_required?: boolean
  process: ProcessStats | null
  lifecycle_state: string
  restarts_total?: number
  capability_violations_total?: number
  tool_calls_total?: number
  tool_errors_total?: number
  rest_requests_total?: number
  worker_runs_total?: number
  worker_failures_total?: number
  events_dropped_total?: number
  /** Times the supervisor SIGKILL'd this extension's whole process group
   * because the child or a grandchild leaked. Rising → process leak. */
  group_kills_total?: number
}

interface ReapedGroup {
  extension_id: string
  pgid: number
  was_alive: boolean
}

/** The boot-reaper block. The wire payload carries ONLY `groups`;
 * `total`/`killed` are methods on the Rust side and are not serialized,
 * so we derive them in the UI from `groups`. */
interface ReaperReport {
  groups: ReapedGroup[]
}

interface SupervisorHealth {
  reaped: ReaperReport | null
}

// ---------- data hooks --------------------------------------------

/** Fetch the per-extension overview rows. This ALWAYS drives the table. */
function useExtensionOverview() {
  const client = useRubixClient()
  return useQuery<SupervisorExtensionRow[], Error>({
    queryKey: ['admin', 'supervisor', 'overview'],
    refetchInterval: 5000,
    queryFn: async () => {
      const base = client.starter.baseUrl
      const res = await client.starter.fetch(
        `${base}/api/v1/extensions/overview`,
        { method: 'GET', headers: { accept: 'application/json' } },
      )
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const rows = (await res.json()) as SupervisorExtensionRow[]
      return rows ?? []
    },
  })
}

/** Fetch the supervisor health (boot-reaper) block.
 *
 * Resolves to `{ reaped }`. This is treated defensively: a 404 (or any
 * other error) resolves to `{ reaped: null }` so the reaper card can show
 * a "telemetry unavailable" note WITHOUT failing the page or the table. */
function useSupervisorHealth() {
  const client = useRubixClient()
  return useQuery<SupervisorHealth, Error>({
    queryKey: ['admin', 'supervisor', 'health'],
    refetchInterval: 5000,
    queryFn: async () => {
      const base = client.starter.baseUrl
      try {
        const res = await client.starter.fetch(
          `${base}/api/v1/admin/supervisor/health`,
          { method: 'GET', headers: { accept: 'application/json' } },
        )
        if (res.ok) {
          const body = (await res.json()) as SupervisorHealth
          return { reaped: body.reaped ?? null }
        }
        // Any non-2xx (notably 404) → degrade to an unavailable reaper card.
      } catch {
        // network error → degrade likewise
      }
      return { reaped: null }
    },
  })
}

// ---------- lifecycle badge ---------------------------------------

function LifecycleBadge({ state }: { state: string }) {
  const s = state?.toLowerCase()
  const color =
    s === 'running'
      ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)] ring-[color:var(--color-leaf)]/30'
      : s === 'failed' || s === 'errored'
        ? 'bg-red-500/10 text-red-400 ring-red-500/30'
        : s === 'starting' || s === 'stopping'
          ? 'bg-[color:var(--color-sun)]/15 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/30'
          : 'bg-[color:var(--color-surface-2)]/60 text-[color:var(--color-muted)] ring-[color:var(--color-border)]'
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] font-medium uppercase tracking-wider ring-1 ${color}`}
    >
      <Activity className="h-3 w-3" />
      {state || 'unknown'}
    </span>
  )
}

/** A counter cell that goes amber/red when nonzero, used for the
 * leak-indicating columns (group kills, cap violations, dropped events). */
function CountCell({
  value,
  severity = 'amber',
  title,
}: {
  value?: number
  severity?: 'amber' | 'red'
  title?: string
}) {
  const v = value ?? 0
  if (v === 0) {
    return <span className="text-[color:var(--color-subtle)]">0</span>
  }
  const tone =
    severity === 'red'
      ? 'bg-red-500/15 text-red-400 ring-red-500/30'
      : 'bg-[color:var(--color-sun)]/15 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/30'
  return (
    <span
      title={title}
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold tabular-nums ring-1 ${tone}`}
    >
      {v}
    </span>
  )
}

// ---------- boot reaper card --------------------------------------

function ReaperCard({ reaped }: { reaped: ReaperReport | null }) {
  const intl = useIntl()
  const tr = (
    id: string,
    def: string,
    values?: Record<string, string | number>,
  ) => intl.formatMessage({ id, defaultMessage: def }, values)

  // No health endpoint (fallback path) → we genuinely don't know the
  // reaper state. Render an explicit "unavailable" note rather than a
  // misleading "clean boot".
  if (!reaped) {
    return (
      <div className="glass mb-6 rounded-2xl px-5 py-4">
        <div className="flex items-center gap-2">
          <HeartPulse className="h-4 w-4 text-[color:var(--color-subtle)]" />
          <h2 className="text-sm font-semibold tracking-tight">
            {tr('supervisor.reaper.title', 'Boot reaper')}
          </h2>
        </div>
        <p className="mt-1 text-xs leading-relaxed text-[color:var(--color-muted)]">
          {tr(
            'supervisor.reaper.unavailable',
            'Reaper telemetry is unavailable (the supervisor health endpoint did not respond). Showing extension stats from the overview projection.',
          )}
        </p>
      </div>
    )
  }

  // `total`/`killed` are not on the wire — derive them from `groups`.
  const total = reaped.groups.length
  const killed = reaped.groups.filter((g) => g.was_alive).length
  const clean = total === 0

  return (
    <div
      className={`glass mb-6 rounded-2xl px-5 py-4 ring-1 ${
        clean
          ? 'ring-[color:var(--color-leaf)]/30'
          : 'ring-[color:var(--color-sun)]/30'
      }`}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-2">
          {clean ? (
            <ShieldCheck className="h-4 w-4 text-[color:var(--color-leaf)]" />
          ) : (
            <ShieldAlert className="h-4 w-4 text-[color:var(--color-sun)]" />
          )}
          <h2 className="text-sm font-semibold tracking-tight">
            {tr('supervisor.reaper.title', 'Boot reaper')}
          </h2>
        </div>
        {clean ? (
          <span className="inline-flex items-center gap-1.5 rounded-full bg-[color:var(--color-leaf)]/15 px-2.5 py-0.5 text-[11px] font-medium uppercase tracking-wider text-[color:var(--color-leaf)] ring-1 ring-[color:var(--color-leaf)]/30">
            <ShieldCheck className="h-3 w-3" />
            {tr('supervisor.reaper.clean', 'No orphaned processes — clean boot')}
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5 rounded-full bg-[color:var(--color-sun)]/15 px-2.5 py-0.5 text-[11px] font-medium uppercase tracking-wider text-[color:var(--color-sun)] ring-1 ring-[color:var(--color-sun)]/30">
            <ShieldAlert className="h-3 w-3" />
            {tr('supervisor.reaper.killed', '{killed} of {total} reclaimed', {
              killed,
              total,
            })}
          </span>
        )}
      </div>
      <p className="mt-2 max-w-2xl text-xs leading-relaxed text-[color:var(--color-muted)]">
        {tr(
          'supervisor.reaper.desc',
          'Child process groups left alive by a previously killed agent instance, reclaimed at startup.',
        )}
      </p>

      {!clean && reaped.groups.length > 0 ? (
        <ul className="mt-3 space-y-1">
          {reaped.groups.map((g, i) => (
            <li
              key={`${g.extension_id}-${g.pgid}-${i}`}
              className="flex items-center gap-2 rounded-md bg-[color:var(--color-surface-2)]/40 px-3 py-1.5 text-xs ring-1 ring-[color:var(--color-border)]"
            >
              <span className="font-mono text-[color:var(--color-text)]">{g.extension_id}</span>
              <span className="text-[color:var(--color-subtle)]">pgid</span>
              <span className="font-mono tabular-nums text-[color:var(--color-muted)]">{g.pgid}</span>
              <span
                className={`ml-auto inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider ring-1 ${
                  g.was_alive
                    ? 'bg-[color:var(--color-sun)]/15 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/30'
                    : 'bg-[color:var(--color-surface-2)]/60 text-[color:var(--color-muted)] ring-[color:var(--color-border)]'
                }`}
              >
                {g.was_alive
                  ? tr('supervisor.reaper.wasAlive', 'was alive')
                  : tr('supervisor.reaper.alreadyGone', 'already gone')}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  )
}

// ---------- main panel --------------------------------------------

function SupervisorPanel() {
  const intl = useIntl()
  const tr = (
    id: string,
    def: string,
    values?: Record<string, string | number>,
  ) => intl.formatMessage({ id, defaultMessage: def }, values)

  const overview = useExtensionOverview()
  const health = useSupervisorHealth()
  const rows = useMemo(
    () => overview.data ?? [],
    [overview.data],
  )

  const totalGroupKills = useMemo(
    () => rows.reduce((acc, r) => acc + (r.group_kills_total ?? 0), 0),
    [rows],
  )

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-6 flex items-end justify-between gap-4">
        <div>
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('supervisor.eyebrow', 'Admin')}
            </span>
          </div>
          <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
            {tr('supervisor.title', 'Supervisor Health')}
          </h1>
          <p className="mt-2 max-w-2xl text-sm text-[color:var(--color-muted)]">
            {tr(
              'supervisor.subtitle',
              'Extension supervisor runtime stats, with the orphaned-child-process leak the supervisor now guards against front and centre.',
            )}
          </p>
        </div>
        <div className="text-xs text-[color:var(--color-subtle)]">
          {tr('supervisor.streamStatus', 'Live status')}:{' '}
          <span className="text-[color:var(--color-text)]">poll · 5s</span>
        </div>
      </header>

      {overview.isLoading ? (
        <div className="space-y-4">
          <Skeleton className="h-24 w-full rounded-2xl" />
          <Skeleton className="h-64 w-full rounded-3xl" />
        </div>
      ) : overview.error ? (
        <div className="glass rounded-2xl px-5 py-4">
          <p className="text-sm text-red-400">
            {tr('supervisor.error', 'Failed to load supervisor health')}:{' '}
            {String(overview.error.message)}
          </p>
        </div>
      ) : (
        <>
          {/* Reaper card is driven by the (defensive) health query and
              degrades to an "unavailable" note on its own; the table below
              always renders from the overview query. */}
          <ReaperCard reaped={health.data?.reaped ?? null} />

          {/* group-kill aggregate banner — the leak headline */}
          {totalGroupKills > 0 ? (
            <div className="mb-4 flex items-center gap-2 rounded-2xl bg-red-500/10 px-5 py-3 text-xs ring-1 ring-red-500/30">
              <ShieldAlert className="h-4 w-4 text-red-400" />
              <span className="text-[color:var(--color-text)]">
                {tr(
                  'supervisor.groupKills.banner',
                  '{n} process-group force-kill(s) across all extensions — at least one extension is leaking child processes.',
                  { n: totalGroupKills },
                )}
              </span>
            </div>
          ) : null}

          <div className="glass overflow-x-auto rounded-3xl">
            <table className="w-full min-w-[920px] border-collapse text-sm">
              <thead>
                <tr className="border-b border-[color:var(--color-border)] text-left text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                  <th className="px-4 py-3">{tr('supervisor.col.id', 'Extension')}</th>
                  <th className="px-4 py-3">{tr('supervisor.col.state', 'State')}</th>
                  <th className="px-4 py-3 text-right">{tr('supervisor.col.pid', 'PID')}</th>
                  <th className="px-4 py-3 text-right">{tr('supervisor.col.uptime', 'Uptime')}</th>
                  <th className="px-4 py-3 text-right">{tr('supervisor.col.rss', 'RSS')}</th>
                  <th className="px-4 py-3 text-right">{tr('supervisor.col.cpu', 'CPU')}</th>
                  <th className="px-4 py-3 text-right">{tr('supervisor.col.restarts', 'Restarts')}</th>
                  <th className="px-4 py-3 text-right">
                    {tr('supervisor.col.groupKills', 'Group kills')}
                  </th>
                  <th className="px-4 py-3 text-right">
                    {tr('supervisor.col.capViolations', 'Cap. violations')}
                  </th>
                  <th className="px-4 py-3 text-right">
                    {tr('supervisor.col.dropped', 'Dropped events')}
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows.length === 0 ? (
                  <tr>
                    <td
                      colSpan={10}
                      className="px-4 py-10 text-center text-sm text-[color:var(--color-subtle)]"
                    >
                      {tr('supervisor.empty', 'No extensions registered.')}
                    </td>
                  </tr>
                ) : (
                  rows.map((r) => {
                    const name = r.display_name ?? r.id
                    return (
                      <tr
                        key={r.id}
                        className="border-b border-[color:var(--color-border)]/50 last:border-b-0 hover:bg-[color:var(--color-surface-2)]/30"
                      >
                        <td className="px-4 py-3">
                          <div className="font-medium text-[color:var(--color-text)]">{name}</div>
                          <div className="font-mono text-[10px] text-[color:var(--color-subtle)]">{r.id}</div>
                        </td>
                        <td className="px-4 py-3">
                          <LifecycleBadge state={r.lifecycle_state} />
                        </td>
                        <td className="px-4 py-3 text-right font-mono tabular-nums text-[color:var(--color-muted)]">
                          {r.process?.pid ?? '—'}
                        </td>
                        <td className="px-4 py-3 text-right tabular-nums text-[color:var(--color-muted)]">
                          {r.process?.uptime
                            ? formatUptime(r.process.uptime)
                            : '—'}
                        </td>
                        <td className="px-4 py-3 text-right tabular-nums text-[color:var(--color-muted)]">
                          {r.process ? formatBytes(r.process.rss_bytes) : '—'}
                        </td>
                        <td className="px-4 py-3 text-right tabular-nums text-[color:var(--color-muted)]">
                          {r.process?.cpu_pct != null
                            ? `${r.process.cpu_pct.toFixed(1)}%`
                            : '—'}
                        </td>
                        <td className="px-4 py-3 text-right tabular-nums text-[color:var(--color-muted)]">
                          {r.restarts_total ?? 0}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <CountCell
                            value={r.group_kills_total}
                            severity="red"
                            title={tr(
                              'supervisor.groupKills.tooltip',
                              "Times the supervisor force-killed this extension's process group — indicates leaked child processes",
                            )}
                          />
                        </td>
                        <td className="px-4 py-3 text-right">
                          <CountCell value={r.capability_violations_total} severity="amber" />
                        </td>
                        <td className="px-4 py-3 text-right">
                          <CountCell value={r.events_dropped_total} severity="amber" />
                        </td>
                      </tr>
                    )
                  })
                )}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  )
}

function SupervisorRoute() {
  return (
    <ErrorBoundary>
      <SupervisorPanel />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/supervisor')({
  component: SupervisorRoute,
})
