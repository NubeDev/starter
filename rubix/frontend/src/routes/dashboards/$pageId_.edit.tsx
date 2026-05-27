// `/dashboards/$pageId/edit` — Puck visual editor for one dashboard.
//
// Loader fetches the live page via `rubix.dashboard.get`, then hands
// the IR `ComponentTree` + `revision_id` to `<PuckBuilder>`. Save is
// wired through `makeRubixSaveTransport`, which maps a 409 into the
// builder's conflict modal. Discard re-fetches and remounts.

import { createFileRoute, Link } from '@tanstack/react-router'
import {
  PuckBuilder,
  catalogueFromMap,
  makeRubixSaveTransport,
  type Catalogue,
  type CatalogueEntry,
  type ComponentTree,
  type PuckBuilderHandle,
  type PuckSaveTransport,
} from '@nube/starter-ui-sdui-puck'
import { SduiPage } from '@nube/starter-ui-sdui-react'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  useRubixClient,
  useTenantList,
  usePageLiveness,
} from '@nube/rubix-client-react'
import { useAuth } from '@nube/starter-client-react'
import { ErrorBoundary } from '@/components/error-boundary'
import { useIntl } from 'react-intl'
import { Columns2, Eye, PanelLeft, RefreshCw, Save } from 'lucide-react'

function EditDashboardRoute() {
  const intl = useIntl()
  const { pageId } = Route.useParams()
  const pageRef = pageId.includes('.') ? pageId : `dashboard.${pageId}`
  const client = useRubixClient()
  const { user } = useAuth()

  // Active tenant for the session. The app convention (see
  // `top-header.tsx`) is to treat the first entry returned by
  // `rubix.tenant.list` as the active tenant — `MeResponse` does
  // not carry a tenant field, so this list is the only session-
  // scoped source we have. The dashboard fetch + save transport
  // both stay disabled until this resolves.
  const tenantListQuery = useTenantList()
  const activeTenantId = tenantListQuery.data?.tenants?.[0]?.tenant_id ?? ''

  // §B3 catalogue seam. Each kind hits a rubix verb (or the
  // /api/v1/tools index for the tool catalogue). Failures bubble up
  // and the picker degrades to a free-text input + warning. The
  // analytics-template list verb is still under discovery — until it
  // lands the kind rejects and operators get the free-text fallback.
  const catalogue = useMemo<Catalogue>(() => {
    return catalogueFromMap({
      analytics_template: async () => {
        // TODO: replace with the real verb once
        // `rubix.analytics.template.list` (or equivalent) ships. For
        // now we reject and let the picker degrade to a text input.
        throw new Error(
          'analytics-template list verb not yet wired — type the template stem',
        )
      },
      tool: async () => {
        const res = await fetch('/api/v1/tools', {
          credentials: 'include',
        })
        if (!res.ok) throw new Error(`/api/v1/tools → ${res.status}`)
        const body = (await res.json()) as {
          tools?: Array<{ name?: string; description?: string }>
        }
        const tools = Array.isArray(body.tools) ? body.tools : []
        return tools
          .filter((t): t is { name: string; description?: string } =>
            typeof t.name === 'string',
          )
          .map<CatalogueEntry>((t) => ({
            value: t.name,
            label: t.name,
            hint: t.description,
          }))
      },
      tenant: async () => {
        const res = await client.tenantList({})
        return (res.tenants ?? []).map<CatalogueEntry>((t) => ({
          value: t.tenant_id,
          label: t.name || t.tenant_id,
        }))
      },
      unit_symbol: async () => [
        { value: 'kWh', label: 'kWh — kilowatt hours' },
        { value: 'W', label: 'W — watts' },
        { value: 'L', label: 'L — litres' },
        { value: 'L/min', label: 'L/min — litres per minute' },
        { value: '°C', label: '°C — degrees Celsius' },
        { value: '%', label: '% — percent' },
        { value: 'kPa', label: 'kPa — kilopascals' },
      ],
      page_state_key: async () => [
        { value: '$page.range_from', label: '$page.range_from' },
        { value: '$page.range_to', label: '$page.range_to' },
      ],
    })
  }, [client])

  // §B6 runtime schema-hash banner. The route fetches the schema
  // hash the live rubix-agent was built against and hands it to
  // `<PuckBuilder>`; the builder compares against its bundled hash
  // (`IR_SCHEMA_HASH`) and surfaces a non-blocking banner inside
  // the canvas when they diverge — so the operator knows the
  // palette is stale without us blocking the edit. The fetch is
  // best-effort: when the verb doesn't exist on the agent (404,
  // network error, missing key) we silently skip the banner; the
  // CI drift guard at `packages/starter-ui-sdui-puck/scripts/check-
  // schema-drift.mjs` is the belt-and-braces at PR time.
  //
  // Discovery: as of 2026-05-26 the rubix-agent does not yet expose
  // a dedicated schema-hash verb. The fetch attempts the
  // proposed endpoint `GET /api/v1/ui/schema/hash` (see
  // `rubix/docs/design/sdui/components/README.md`); if it 404s the
  // banner stays dormant.
  const [liveSchemaHash, setLiveSchemaHash] = useState<string | undefined>(
    undefined,
  )
  useEffect(() => {
    let cancelled = false
    fetch('/api/v1/ui/schema/hash', { credentials: 'include' })
      .then(async (res) => {
        if (!res.ok) return undefined
        const body = (await res.json()) as { hash?: unknown }
        return typeof body.hash === 'string' ? body.hash : undefined
      })
      .catch(() => undefined)
      .then((hash) => {
        if (cancelled) return
        if (hash) setLiveSchemaHash(hash)
      })
    return () => {
      cancelled = true
    }
  }, [])

  // `reloadKey` forces a fresh fetch + builder remount when the
  // operator clicks "Discard my edits" in the conflict modal.
  const [reloadKey, setReloadKey] = useState(0)
  const [page, setPage] = useState<
    | { kind: 'loading' }
    | { kind: 'error'; message: string }
    | { kind: 'ready'; tree: ComponentTree; revisionId: string }
  >({ kind: 'loading' })

  useEffect(() => {
    if (!activeTenantId) return
    let cancelled = false
    setPage({ kind: 'loading' })
    client
      .dashboardGet({ tenant_id: activeTenantId, page_id: pageRef })
      .then((res) => {
        if (cancelled) return
        setPage({
          kind: 'ready',
          tree: res.body_json as ComponentTree,
          revisionId: res.revision_id ?? '',
        })
      })
      .catch((e: { message?: string }) => {
        if (cancelled) return
        setPage({ kind: 'error', message: e.message ?? String(e) })
      })
    return () => {
      cancelled = true
    }
  }, [client, pageRef, reloadKey, activeTenantId])

  // Synchronous discard bridge — invoked by `<PuckBuilder>` when
  // the operator picks "Discard my edits" in the conflict modal.
  // Bumps `reloadKey` so the loader effect re-fetches and the
  // builder remounts with the fresh `initialTree`.
  const handleDiscard = useCallback(() => {
    setReloadKey((k) => k + 1)
  }, [])

  // Live-preview wiring. The right-hand pane mounts `<SduiPage>`
  // against the saved revision of this page. After every successful
  // save we invalidate the `useSduiResolve` query for this pageRef
  // so the preview re-fetches and repaints in place — operator gets
  // instant feedback without a navigation. Refresh button does the
  // same on demand. `previewKey` is bumped after save as a belt-and-
  // braces remount in case the page subtree holds local state we'd
  // want reset (e.g. zoomed-in µPlot ranges).
  const queryClient = useQueryClient()
  const [previewKey, setPreviewKey] = useState(0)
  const refreshPreview = useCallback(() => {
    queryClient.invalidateQueries({
      predicate: (q) => {
        const key = q.queryKey
        if (!Array.isArray(key) || key[0] !== 'sdui' || key[1] !== 'resolve') {
          return false
        }
        const req = key[2] as { page_ref?: string } | undefined
        return req?.page_ref === pageRef
      },
    })
    setPreviewKey((k) => k + 1)
  }, [queryClient, pageRef])

  const puckRef = useRef<PuckBuilderHandle>(null)
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')

  const handleSave = useCallback(async () => {
    if (!puckRef.current) return
    setSaveState('saving')
    await puckRef.current.save()
    setSaveState(puckRef.current.saveStateKind === 'error' ? 'error' : 'saved')
    if (puckRef.current.saveStateKind !== 'error') {
      setTimeout(() => setSaveState('idle'), 2000)
    }
  }, [])

  const saveTransport: PuckSaveTransport | undefined = useMemo(() => {
    if (!activeTenantId) return undefined
    const base = makeRubixSaveTransport(
      client,
      activeTenantId,
      user?.subject ?? 'unknown',
    )
    return async (req) => {
      const out = await base(req)
      if (out.kind === 'saved') refreshPreview()
      return out
    }
  }, [client, activeTenantId, user?.subject, refreshPreview])

  // View mode for the editor / preview pair. Three modes:
  //   - editor:  Puck only, full width (the calm default).
  //   - split:   Puck + preview side-by-side (good for wide screens).
  //   - preview: full-width Puck stays mounted in the background; the
  //              preview slides in as an overlay panel on top so the
  //              user can flip back without losing Puck state.
  // The last-selected mode is persisted per browser in localStorage
  // so operators don't have to re-pick on every visit.
  type ViewMode = 'editor' | 'split' | 'preview'
  const VIEW_MODE_KEY = 'rubix.editDashboard.viewMode'
  const [viewMode, setViewModeState] = useState<ViewMode>(() => {
    if (typeof window === 'undefined') return 'editor'
    const v = window.localStorage.getItem(VIEW_MODE_KEY)
    return v === 'split' || v === 'preview' || v === 'editor' ? v : 'editor'
  })
  const setViewMode = useCallback((m: ViewMode) => {
    setViewModeState(m)
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(VIEW_MODE_KEY, m)
    }
    // Re-resolve the preview every time the user opens it so they
    // see the latest saved revision without an extra click.
    if (m !== 'editor') refreshPreview()
  }, [refreshPreview])

  // Scope 11 — per-page liveness. When the rubix-agent SSE channel
  // announces a new revision for this `pageRef` (someone else, or
  // the AI assistant, saved while we were editing), surface the
  // same conflict modal the 409-on-save path uses, pre-emptively,
  // so the operator gets the Discard / Keep-editing choice before
  // they hit Save. The 409 path stays as the safety net.
  const liveness = usePageLiveness(pageRef)

  return (
    <ErrorBoundary>
      <section className="flex h-[calc(100vh-4rem)] flex-col">
        <header className="flex items-center justify-between gap-4 border-b border-border bg-background px-4 py-2">
          <div className="flex items-center gap-3 text-sm">
            <Link
              to="/dashboards/$pageId"
              params={{ pageId }}
              className="text-muted-foreground hover:text-foreground"
            >
              ← Back to dashboard
            </Link>
            <span className="text-muted-foreground">/</span>
            <code className="text-foreground">{pageRef}</code>
          </div>
          <div className="flex items-center gap-2">
            {saveState === 'saved' && (
              <span className="text-xs text-green-600">
                {intl.formatMessage({ id: 'edit.saved', defaultMessage: 'Saved' })}
              </span>
            )}
            {saveState === 'error' && (
              <span className="text-xs text-destructive">
                {intl.formatMessage({ id: 'edit.saveFailed', defaultMessage: 'Save failed' })}
              </span>
            )}
            {/* Segmented control — Save + view-mode icons grouped in one pill */}
            <div
              role="tablist"
              aria-label="View mode"
              className="inline-flex items-center rounded-md border border-border bg-background p-0.5"
            >
              {saveTransport && (
                <button
                  type="button"
                  onClick={handleSave}
                  disabled={saveState === 'saving'}
                  title={intl.formatMessage({ id: 'edit.save', defaultMessage: 'Save' })}
                  className="flex items-center gap-1.5 rounded px-2.5 py-1 text-sm font-medium bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 mr-0.5"
                >
                  <Save size={14} />
                  {saveState === 'saving'
                    ? intl.formatMessage({ id: 'edit.saving', defaultMessage: 'Saving…' })
                    : intl.formatMessage({ id: 'edit.save', defaultMessage: 'Save' })}
                </button>
              )}
              {(
                [
                  {
                    id: 'editor' as const,
                    icon: <PanelLeft size={16} />,
                    titleKey: 'edit.viewMode.editor',
                    defaultTitle: 'Editor — edit layout in Puck',
                  },
                  {
                    id: 'split' as const,
                    icon: <Columns2 size={16} />,
                    titleKey: 'edit.viewMode.split',
                    defaultTitle: 'Split — editor and live preview side-by-side',
                  },
                  {
                    id: 'preview' as const,
                    icon: <Eye size={16} />,
                    titleKey: 'edit.viewMode.preview',
                    defaultTitle: 'Preview — full-width live preview of saved revision',
                  },
                ]
              ).map((opt) => (
                <button
                  key={opt.id}
                  type="button"
                  role="tab"
                  aria-selected={viewMode === opt.id}
                  title={intl.formatMessage({ id: opt.titleKey, defaultMessage: opt.defaultTitle })}
                  onClick={() => setViewMode(opt.id)}
                  className={
                    viewMode === opt.id
                      ? 'flex items-center rounded p-1.5 bg-foreground text-background'
                      : 'flex items-center rounded p-1.5 text-muted-foreground hover:text-foreground'
                  }
                >
                  {opt.icon}
                </button>
              ))}
              {viewMode !== 'editor' && (
                <button
                  type="button"
                  onClick={refreshPreview}
                  title={intl.formatMessage({ id: 'edit.refreshPreview', defaultMessage: 'Refresh — re-fetch preview against latest saved revision' })}
                  className="flex items-center rounded p-1.5 text-muted-foreground hover:text-foreground ml-0.5"
                >
                  <RefreshCw size={16} />
                </button>
              )}
            </div>
          </div>
        </header>
        <div className="relative flex flex-1 min-h-0 gap-4 bg-background p-2">
          {!activeTenantId ? (
            tenantListQuery.isError ? (
              <div className="p-6 text-sm text-destructive">
                Failed to resolve active tenant:{' '}
                {tenantListQuery.error?.message ?? 'unknown error'}
              </div>
            ) : tenantListQuery.isSuccess ? (
              <div className="p-6 text-sm text-destructive">
                No tenant is available for this session — ask an admin
                to grant you access.
              </div>
            ) : (
              <div className="p-6 text-sm text-muted-foreground">Resolving tenant…</div>
            )
          ) : page.kind === 'loading' ? (
            <div className="p-6 text-sm text-muted-foreground">Loading…</div>
          ) : page.kind === 'error' ? (
            <div className="p-6 text-sm text-destructive">
              Failed to load {pageRef}: {page.message}
            </div>
          ) : (
            <>
              {/* Puck stays mounted in every mode so toggling doesn't
                  drop unsaved tree state. In `preview` mode we hide
                  it visually but keep it in the tree. */}
              <div
                className={
                  viewMode === 'preview'
                    ? 'hidden'
                    : 'flex-1 min-w-0 min-h-0 overflow-hidden rounded-md border border-border'
                }
              >
                <PuckBuilder
                  ref={puckRef}
                  key={reloadKey}
                  pageRef={pageRef}
                  initialTree={page.tree}
                  initialRevisionId={page.revisionId}
                  onSave={saveTransport}
                  onDiscardRequested={handleDiscard}
                  catalogue={catalogue}
                  liveSchemaHash={liveSchemaHash}
                  liveRevisionId={liveness.latestRevisionId}
                  liveChangeToken={liveness.changeToken}
                  liveActorKind={liveness.actorKind}
                />
              </div>

              {/* Split-mode preview pane — inline next to Puck. */}
              {viewMode === 'split' && (
                <aside className="w-2/5 min-w-[320px] min-h-0 overflow-auto rounded-md border border-border bg-muted">
                  <div className="border-b border-border bg-card px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    Live preview · saved revision
                  </div>
                  <div className="p-4">
                    <SduiPage key={previewKey} pageRef={pageRef} />
                  </div>
                </aside>
              )}

              {/* Preview-mode pane — takes the full content area. The
                  Puck builder above stays mounted (display: none) so
                  flipping back is instant. */}
              {viewMode === 'preview' && (
                <aside className="flex-1 min-w-0 min-h-0 overflow-auto rounded-md border border-border bg-muted">
                  <div className="border-b border-border bg-card px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    Live preview · saved revision
                  </div>
                  <div className="p-4">
                    <SduiPage key={previewKey} pageRef={pageRef} />
                  </div>
                </aside>
              )}
            </>
          )}
        </div>
      </section>
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/dashboards/$pageId_/edit')({
  component: EditDashboardRoute,
})
