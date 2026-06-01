import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { LayoutDashboard, Inbox } from 'lucide-react'
import { useRefreshKey } from '../api/refresh'
import { sitesList, pagesList, widgetsByPage } from '../api/bc'
import { Field, Picker } from '../components/FormKit'
import { PageHeader } from '../components/ui'
import { WidgetTile } from './widgets/WidgetTile'
import { useLook } from '../theme/useLook'
import type { PageRow, SiteRow, WidgetRow } from '../api/bc-types'

// The client view: pick a Site → one of its pages → render the widget tiles.
// The payoff screen. `initialPageId` lets the scan-flow deep-link straight to
// the page it just provisioned onto.
export function PagePreview({ initialPageId }: { initialPageId?: string }) {
  const look = useLook()
  const refresh = useRefreshKey()
  const [sites, setSites] = useState<ReadonlyArray<SiteRow>>([])
  const [pages, setPages] = useState<ReadonlyArray<PageRow>>([])
  const [widgets, setWidgets] = useState<ReadonlyArray<WidgetRow>>([])
  const [siteId, setSiteId] = useState('')
  const [pageId, setPageId] = useState(initialPageId ?? '')

  useEffect(() => {
    sitesList().then(setSites).catch(() => {})
  }, [refresh])

  // When deep-linked to a page, discover its site so the pickers stay coherent.
  useEffect(() => {
    if (!initialPageId || siteId) return
    sitesList()
      .then(async (ss) => {
        for (const s of ss) {
          const ps = await pagesList(s.site_id).catch(() => [])
          if (ps.some((p) => p.page_id === initialPageId)) {
            setSiteId(s.site_id)
            return
          }
        }
      })
      .catch(() => undefined)
  }, [initialPageId, siteId])

  useEffect(() => {
    const p = siteId ? pagesList(siteId) : Promise.resolve([])
    p.then(setPages).catch(() => {})
  }, [siteId, refresh])

  useEffect(() => {
    const p = pageId ? widgetsByPage(pageId) : Promise.resolve([])
    p.then(setWidgets).catch(() => {})
  }, [pageId, refresh])

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <PageHeader eyebrow="Client view" title="Page preview" />

      <div className="mb-6 flex flex-col gap-4">
        <Field label="Site">
          <Picker
            value={siteId}
            placeholder="Choose a site"
            options={sites.map((s) => ({ value: s.site_id, label: s.name }))}
            onChange={(v) => {
              setSiteId(v)
              setPageId('')
            }}
          />
        </Field>
        {siteId && (
          <Field label="Dashboard page">
            <Picker
              value={pageId}
              placeholder="Choose a page"
              options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
              onChange={setPageId}
            />
          </Field>
        )}
      </div>

      {pageId ? (
        widgets.length ? (
          <div className="grid grid-cols-2 gap-gutter">
            {widgets.map((w, i) => (
              <motion.div
                key={w.widget_id}
                initial={{ opacity: 0, y: 16 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.05, type: 'spring', stiffness: 300, damping: 30 }}
                className={w.widget === 'gauge' || w.widget === 'line' ? 'col-span-2' : ''}
              >
                <WidgetTile
                  widget={w.widget}
                  title={w.title ?? w.role ?? `Point ${i + 1}`}
                  accent={look.accent}
                  seed={i + 1}
                />
              </motion.div>
            ))}
          </div>
        ) : (
          <Empty icon={Inbox} text="No widgets on this page yet. Provision a device to populate it." />
        )
      ) : (
        <Empty icon={LayoutDashboard} text="Pick a site and a page to see its sensor tiles." />
      )}
    </div>
  )
}

function Empty({ icon: Icon, text }: { icon: typeof Inbox; text: string }) {
  return (
    <div className="glass mt-4 flex flex-col items-center gap-3 rounded-2xl px-6 py-12 text-center text-ink-muted">
      <Icon className="h-8 w-8" />
      <p className="max-w-[240px] text-sm">{text}</p>
    </div>
  )
}
