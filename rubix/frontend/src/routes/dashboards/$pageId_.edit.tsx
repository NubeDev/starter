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
} from '@nube/starter-ui-sdui-puck'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useRubixClient, useTenantList } from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

function EditDashboardRoute() {
  const { pageId } = Route.useParams()
  const pageRef = pageId.includes('.') ? pageId : `dashboard.${pageId}`
  const client = useRubixClient()

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

  return (
    <ErrorBoundary>
      <section className="flex h-[calc(100vh-4rem)] flex-col">
        <header className="flex items-center justify-between border-b border-slate-200 px-4 py-2">
          <div className="flex items-center gap-3 text-sm">
            <Link
              to="/dashboards/$pageId"
              params={{ pageId }}
              className="text-slate-600 hover:text-slate-900"
            >
              ← Back to dashboard
            </Link>
            <span className="text-slate-400">/</span>
            <code className="text-slate-700">{pageRef}</code>
          </div>
        </header>
        <div className="flex-1 min-h-0">
          {!activeTenantId ? (
            tenantListQuery.isError ? (
              <div className="p-6 text-sm text-red-600">
                Failed to resolve active tenant:{' '}
                {tenantListQuery.error?.message ?? 'unknown error'}
              </div>
            ) : tenantListQuery.isSuccess ? (
              <div className="p-6 text-sm text-red-600">
                No tenant is available for this session — ask an admin
                to grant you access.
              </div>
            ) : (
              <div className="p-6 text-sm text-slate-500">Resolving tenant…</div>
            )
          ) : page.kind === 'loading' ? (
            <div className="p-6 text-sm text-slate-500">Loading…</div>
          ) : page.kind === 'error' ? (
            <div className="p-6 text-sm text-red-600">
              Failed to load {pageRef}: {page.message}
            </div>
          ) : (
            <PuckBuilder
              key={reloadKey}
              pageRef={pageRef}
              initialTree={page.tree}
              initialRevisionId={page.revisionId}
              onSave={makeRubixSaveTransport(client, activeTenantId)}
              onDiscardRequested={handleDiscard}
              catalogue={catalogue}
            />
          )}
        </div>
      </section>
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/dashboards/$pageId_/edit')({
  component: EditDashboardRoute,
})
