// `/extensions` — installed-extension admin surface.
//
// This route is the SSE worked example for the rubix-frontend-wire
// job: a `useExtensionsList()` table acts as the source of truth for
// row shape while a sibling `useExtensionEvents()` subscription
// overlays live lifecycle state on top, so toggling start/stop
// updates the badge without waiting for the next list refetch.
//
// Start / Stop / Restart buttons call the matching
// `useExtensionStart`/`useExtensionStop`/`useExtensionRestart`
// mutations; success auto-invalidates `['rubix','extensions']` from
// inside the hook so the table reflects authoritative state.

import { Link, Outlet, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo, useState, useSyncExternalStore } from 'react'
import { useIntl } from 'react-intl'
import {
  Activity,
  Boxes,
  CheckCircle2,
  Download,
  MoreVertical,
  Power,
  PowerOff,
  Settings,
  Trash2,
} from 'lucide-react'
import {
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Skeleton,
} from '@nube/starter-ui-kit'
import {
  useExtensionsList,
  useExtensionEnable,
  useExtensionDisable,
  useExtensionProcess,
  useExtensionMetrics,
  type ExtensionContributesSummary,
  type ExtensionSummary,
} from '@nube/rubix-client-react'
import {
  useExtensionHostManager,
  type ExtensionRemoteFactory,
} from '@nube/starter-ext-ui'
import { ErrorBoundary } from '@/components/error-boundary'
import { getExtensionHost } from '@/lib/extension-host'
import {
  UninstallDialog,
  formatBytes,
  formatUptime,
} from '@/components/extensions/uninstall-dialog'

function StatusBadge({ state }: { state: string }) {
  const color =
    state === 'running'
      ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)] ring-[color:var(--color-leaf)]/30'
      : state === 'errored'
        ? 'bg-red-500/10 text-red-400 ring-red-500/30'
        : state === 'starting' || state === 'stopping'
          ? 'bg-[color:var(--color-sun)]/15 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/30'
          : 'bg-[color:var(--color-surface-2)]/60 text-[color:var(--color-muted)] ring-[color:var(--color-border)]'
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] font-medium uppercase tracking-wider ring-1 ${color}`}
    >
      <Activity className="h-3 w-3" />
      {state}
    </span>
  )
}

export function ExtensionsTable() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useExtensionsList()
  const enableMut = useExtensionEnable()
  const disableMut = useExtensionDisable()
  const navigate = useNavigate()
  const [uninstallId, setUninstallId] = useState<string | null>(null)

  // No aggregate SSE endpoint today — the agent only exposes per-id
  // streams at `/api/v1/extensions/{id}/events`. Live status is
  // refreshed by list refetch after each mutation.
  const liveStateById = useMemo(() => new Map<string, string>(), [])

  const rows: ExtensionSummary[] = Array.isArray(list.data)
    ? (list.data as unknown as ExtensionSummary[])
    : (list.data?.extensions ?? [])

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8 flex items-end justify-between gap-4">
        <div>
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('extensions.eyebrow', 'Extensions')}
            </span>
          </div>
          <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
            {tr('extensions.title', 'Installed extensions')}
          </h1>
        </div>
        <div className="text-xs text-[color:var(--color-subtle)]">
          {tr('extensions.streamStatus', 'Live status')}: <span className="text-[color:var(--color-text)]">poll</span>
        </div>
      </header>

      <div className="glass overflow-hidden rounded-3xl">
        <div className="grid grid-cols-[1.5fr_1fr_1fr_auto] gap-4 border-b border-[color:var(--color-border)] px-6 py-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          <div>{tr('extensions.col.name', 'Name')}</div>
          <div>{tr('extensions.col.state', 'State')}</div>
          <div>{tr('extensions.col.enabled', 'Enabled')}</div>
          <div className="text-right">{tr('extensions.col.actions', 'Actions')}</div>
        </div>
        {list.isLoading ? (
          <div className="space-y-3 p-4">
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </div>
        ) : rows.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Boxes />
              </EmptyMedia>
              <EmptyTitle>
                {tr('extensions.empty.title', 'No extensions installed')}
              </EmptyTitle>
              <EmptyDescription>
                {tr(
                  'extensions.empty.body',
                  'Install a Rubix extension to surface its pages, widgets, and commands here.',
                )}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          rows.map((row) => {
            const r = row as ExtensionSummary & {
              display_name?: string | null
              version?: string | null
            }
            const state = liveStateById.get(r.id) ?? r.state
            const name = r.display_name ?? r.name ?? r.id
            const enabled =
              typeof r.enabled === 'boolean' ? r.enabled : r.enabled === 'enabled'
            return (
              <div
                key={r.id}
                className="border-b border-[color:var(--color-border)]/50 last:border-b-0"
              >
                <div className="grid grid-cols-[1.5fr_1fr_1fr_auto] items-center gap-4 px-6 py-4">
                  <div>
                    <Link
                      to="/extensions/$extId/$"
                      params={{ extId: r.id, _splat: '' }}
                      className="font-medium text-[color:var(--color-text)] hover:text-[color:var(--color-leaf)]"
                    >
                      {name}
                      {r.version ? (
                        <span className="ml-2 text-[10px] font-normal text-[color:var(--color-subtle)]">
                          v{r.version}
                        </span>
                      ) : null}
                    </Link>
                    <div className="font-mono text-[10px] text-[color:var(--color-subtle)]">{r.id}</div>
                  </div>
                  <div><StatusBadge state={state} /></div>
                  <div className="text-sm text-[color:var(--color-muted)]">
                    {enabled ? tr('common.yes', 'Yes') : tr('common.no', 'No')}
                  </div>
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={enableMut.isPending || enabled}
                      onClick={() => enableMut.mutate({ id: r.id })}
                    >
                      <Power className="h-3.5 w-3.5" />
                      Enable
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={disableMut.isPending || !enabled}
                      onClick={() => disableMut.mutate({ id: r.id })}
                    >
                      <PowerOff className="h-3.5 w-3.5" />
                      Disable
                    </Button>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          size="sm"
                          variant="outline"
                          aria-label={tr('extensions.rowMenu', 'More actions')}
                        >
                          <MoreVertical className="h-3.5 w-3.5" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onSelect={() =>
                            void navigate({
                              to: '/extensions/$extId/$',
                              params: { extId: r.id, _splat: 'admin' },
                            })
                          }
                        >
                          <Settings className="mr-2 h-3.5 w-3.5" />
                          {tr('extensions.admin.action', 'Admin')}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onSelect={() => setUninstallId(r.id)}
                          className="text-red-400 focus:text-red-400"
                        >
                          <Trash2 className="mr-2 h-3.5 w-3.5" />
                          {tr('extensions.uninstall.action', 'Uninstall')}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </div>
                <ExtensionRowStats id={r.id} />
                <ExtensionContributesPanel id={r.id} contributes={r.contributes} />
              </div>
            )
          })
        )}
      </div>
      {uninstallId ? (
        <UninstallDialog
          extId={uninstallId}
          onClose={() => setUninstallId(null)}
        />
      ) : null}
    </section>
  )
}

/** Inline per-row stats: restarts + violations from the list response
 * (already loaded, no extra fetch), plus a per-row fetch for live process
 * (mem/cpu/uptime) and metrics (tool/REST calls + errors). Per-row hooks
 * mean N requests on initial load, but they're cheap and cached per id. */
function ExtensionRowStats({ id }: { id: string }) {
  const proc = useExtensionProcess(id)
  const metrics = useExtensionMetrics(id)

  const chips: Array<{ label: string; value: string }> = []
  if (proc.data) {
    chips.push({ label: 'uptime', value: formatUptime(proc.data.uptime) })
    chips.push({ label: 'mem', value: formatBytes(proc.data.rss_bytes) })
    if (proc.data.cpu_pct != null)
      chips.push({ label: 'cpu', value: `${proc.data.cpu_pct.toFixed(1)}%` })
  }
  if (metrics.data) {
    const m = metrics.data
    const calls = m.tool_calls_total + m.rest_requests_total + m.worker_runs_total
    const errors = m.tool_errors_total + m.worker_failures_total
    if (calls) chips.push({ label: 'calls', value: String(calls) })
    if (errors) chips.push({ label: 'errors', value: String(errors) })
    if (m.restarts_total) chips.push({ label: 'restarts', value: String(m.restarts_total) })
    if (m.capability_violations_total)
      chips.push({ label: 'cap violations', value: String(m.capability_violations_total) })
  }

  if (chips.length === 0) return null

  return (
    <div className="flex flex-wrap items-center gap-1.5 px-6 pb-2">
      <span className="text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
        Stats
      </span>
      {chips.map((c) => (
        <span
          key={c.label}
          className="inline-flex items-center gap-1 rounded-full bg-[color:var(--color-surface-2)]/60 px-2 py-0.5 text-[11px] text-[color:var(--color-muted)] ring-1 ring-[color:var(--color-border)]"
        >
          <span className="font-medium text-[color:var(--color-text)]">{c.value}</span>
          {c.label}
        </span>
      ))}
    </div>
  )
}

/** `/extensions` and `/extensions/$extId/$rest` share this layout.
 * It only renders `<Outlet />` so child routes (the admin index in
 * `extensions.index.tsx` and the per-extension route in
 * `extensions.$extId.$.tsx`) own the page body. Wrapping in
 * `<ErrorBoundary>` here means a buggy child renders the fallback
 * without unmounting the rest of the layout. */
function ExtensionsRoute() {
  return (
    <ErrorBoundary>
      <Outlet />
    </ErrorBoundary>
  )
}

/** Per-extension "what this contributes" panel + a Load UI button.
 * Reads counts + UI block from the list-row `contributes` summary
 * (server-side: `ContributesSummary` in `starter-ext-server`). On
 * Load UI, dynamic-imports `ui.entry` from `/api/v1/extensions/<id>/ui/<entry>`
 * and registers it with the host manager so `<ExtensionSlot/>` mounts it. */
function ExtensionContributesPanel({
  id,
  contributes,
}: {
  id: string
  contributes?: ExtensionContributesSummary
}) {
  // Subscribe directly to the manager rather than `useExtensionHost()` —
  // the latter also fetches `/extensions` on every mount, which was
  // firing N parallel times for an N-row list. We only need the
  // registered-remotes view here.
  const mgr = useExtensionHostManager()
  const registered = useSyncExternalStore(
    (cb) => mgr.subscribe(cb),
    () => mgr.listRemotes().some((r) => r.id === id),
    () => mgr.listRemotes().some((r) => r.id === id),
  )
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)

  const ui = contributes?.ui ?? null

  const pills = useMemo(() => {
    if (!contributes) return [] as Array<{ label: string; count: number }>
    return [
      { label: 'tools', count: contributes.tools },
      { label: 'cli', count: contributes.cli },
      { label: 'rest', count: contributes.rest },
      { label: 'grpc', count: contributes.grpc },
      { label: 'workers', count: contributes.workers },
      { label: 'nodes', count: contributes.nodes },
      { label: 'skills', count: contributes.skills },
      { label: 'ui slots', count: ui?.exposes?.length ?? 0 },
    ].filter((p) => p.count > 0)
  }, [contributes, ui])

  async function loadUi() {
    if (!ui) return
    setLoading(true)
    setLoadError(null)
    try {
      // The agent's `GET /api/v1/extensions/{id}/ui/{*path}` handler
      // computes a `ui_root` = the parent dir of `manifest.contributes
      // .ui.entry`, then serves the wildcard suffix from that root.
      // For a manifest with `entry: ui/remoteEntry.js` this means the
      // URL for the entry file itself is just the basename
      // (`remoteEntry.js`) — handing it the full `ui/remoteEntry.js`
      // would produce a `/ui/ui/remoteEntry.js` 404.
      const entryBasename = ui.entry
        .replace(/^\/+/, '')
        .split('/')
        .pop()!
      const entryUrl = `/api/v1/extensions/${encodeURIComponent(id)}/ui/${entryBasename}`
      const mod = (await import(/* @vite-ignore */ entryUrl)) as
        | ExtensionRemoteFactory
        | { default: ExtensionRemoteFactory }
      const factory: ExtensionRemoteFactory =
        'init' in mod ? (mod as ExtensionRemoteFactory) : mod.default
      if (!factory || typeof factory.init !== 'function') {
        throw new Error('remoteEntry did not export an ExtensionRemoteFactory')
      }
      await getExtensionHost().registerExtensionRemote(
        id,
        { entry: ui.entry, exposes: ui.exposes ?? [] },
        factory,
      )
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }

  if (!contributes) {
    return null
  }

  return (
    <div className="grid grid-cols-[1fr_auto] items-start gap-4 px-6 pb-4">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          Contributes
        </span>
        {pills.length === 0 ? (
          <span className="text-xs text-[color:var(--color-subtle)]">(nothing)</span>
        ) : (
          pills.map((p) => (
            <span
              key={p.label}
              className="inline-flex items-center gap-1 rounded-full bg-[color:var(--color-surface-2)]/60 px-2 py-0.5 text-[11px] text-[color:var(--color-muted)] ring-1 ring-[color:var(--color-border)]"
            >
              <span className="font-medium text-[color:var(--color-text)]">{p.count}</span>
              {p.label}
            </span>
          ))
        )}
        {loadError ? (
          <span className="ml-2 text-[11px] text-red-400">{loadError}</span>
        ) : null}
      </div>
      <div className="flex justify-end gap-2">
        {registered ? (
          <span className="inline-flex items-center gap-1 rounded-full bg-[color:var(--color-leaf)]/15 px-2 py-0.5 text-[11px] font-medium uppercase tracking-wider text-[color:var(--color-leaf)] ring-1 ring-[color:var(--color-leaf)]/30">
            <CheckCircle2 className="h-3 w-3" />
            UI loaded
          </span>
        ) : null}
        {ui ? (
          <Button
            size="sm"
            variant="default"
            disabled={loading}
            onClick={() => void loadUi()}
            title={registered ? 'Reload UI bundle' : `Load ${ui.entry}`}
          >
            <Download className="h-3.5 w-3.5" />
            {loading
              ? 'Loading…'
              : registered
                ? 'Reload UI'
                : 'Load UI'}
          </Button>
        ) : (
          <span className="text-[11px] text-[color:var(--color-subtle)]">
            No UI bundle
          </span>
        )}
      </div>
    </div>
  )
}

export const Route = createFileRoute('/extensions')({ component: ExtensionsRoute })
