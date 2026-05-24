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

import { createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { useIntl } from 'react-intl'
import { Activity, Play, RotateCw, Square } from 'lucide-react'
import { Button } from '@nube/starter-ui-kit'
import {
  useExtensionsList,
  useExtensionStart,
  useExtensionStop,
  useExtensionRestart,
  useExtensionEvents,
  type ExtensionSummary,
} from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

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

function ExtensionsTable() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useExtensionsList()
  const events = useExtensionEvents()
  const startMut = useExtensionStart()
  const stopMut = useExtensionStop()
  const restartMut = useExtensionRestart()

  // SSE overlay: collapse the lifecycle frames into a per-extension
  // map of the most-recent state so a live status badge can render
  // without waiting for the next list refetch.
  const liveStateById = useMemo(() => {
    const m = new Map<string, string>()
    for (const e of events.events) {
      if (e.kind === 'lifecycle') m.set(e.extension_id, e.state)
    }
    return m
  }, [events.events])

  const rows: ExtensionSummary[] = list.data?.extensions ?? []

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
          {tr('extensions.streamStatus', 'Live status')}: <span className="text-[color:var(--color-text)]">{events.status}</span>
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
          <div className="px-6 py-8 text-sm text-[color:var(--color-muted)]">
            {tr('common.loading', 'Loading…')}
          </div>
        ) : rows.length === 0 ? (
          <div className="px-6 py-8 text-sm text-[color:var(--color-muted)]">
            {tr('extensions.empty', 'No extensions installed.')}
          </div>
        ) : (
          rows.map((row) => {
            const state = liveStateById.get(row.id) ?? row.state
            return (
              <div
                key={row.id}
                className="grid grid-cols-[1.5fr_1fr_1fr_auto] items-center gap-4 border-b border-[color:var(--color-border)]/50 px-6 py-4 last:border-b-0"
              >
                <div>
                  <div className="font-medium text-[color:var(--color-text)]">{row.name}</div>
                  <div className="font-mono text-[10px] text-[color:var(--color-subtle)]">{row.id}</div>
                </div>
                <div><StatusBadge state={state} /></div>
                <div className="text-sm text-[color:var(--color-muted)]">
                  {row.enabled ? tr('common.yes', 'Yes') : tr('common.no', 'No')}
                </div>
                <div className="flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={startMut.isPending || state === 'running' || state === 'starting'}
                    onClick={() => startMut.mutate({ id: row.id })}
                  >
                    <Play className="h-3.5 w-3.5" />
                    {tr('extensions.action.start', 'Start')}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={stopMut.isPending || state === 'stopped' || state === 'stopping'}
                    onClick={() => stopMut.mutate({ id: row.id })}
                  >
                    <Square className="h-3.5 w-3.5" />
                    {tr('extensions.action.stop', 'Stop')}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={restartMut.isPending}
                    onClick={() => restartMut.mutate({ id: row.id })}
                  >
                    <RotateCw className="h-3.5 w-3.5" />
                    {tr('extensions.action.restart', 'Restart')}
                  </Button>
                </div>
              </div>
            )
          })
        )}
      </div>
    </section>
  )
}

function ExtensionsRoute() {
  return (
    <ErrorBoundary>
      <ExtensionsTable />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/extensions')({ component: ExtensionsRoute })
