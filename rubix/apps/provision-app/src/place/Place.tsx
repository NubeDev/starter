import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Loader2, Plus } from 'lucide-react'
import { useLook } from '../theme/useLook'
import { useRefreshKey } from '../api/refresh'
import { sitesList, siteCreate, locationsList, pagesList } from '../api/bc'
import { mintId } from '../api/ids'
import { Field, Picker, TextInput } from '../components/FormKit'
import type { LocationRow, PageRow, SiteRow } from '../api/bc-types'
import { EMPTY_PLACEMENT, type Placement } from './placement'

// Shared placement step: Site (or + create inline) → Location (or + new) →
// Page (scoped to the chosen site, + new). The page picker only appears once a
// site is chosen — a page belongs to a site. Adapted from the extension's
// pwa/place.tsx with the glass FormKit.
export function Place({ value, onChange }: { value: Placement; onChange: (next: Placement) => void }) {
  const look = useLook()
  const refresh = useRefreshKey()
  const [sites, setSites] = useState<ReadonlyArray<SiteRow>>([])
  const [locations, setLocations] = useState<ReadonlyArray<LocationRow>>([])
  const [pages, setPages] = useState<ReadonlyArray<PageRow>>([])
  const [newSite, setNewSite] = useState('')
  const [creatingSite, setCreatingSite] = useState(false)

  const set = (patch: Partial<Placement>) => onChange({ ...value, ...patch })

  useEffect(() => {
    sitesList().then(setSites).catch(() => {})
  }, [refresh])

  useEffect(() => {
    const p = value.siteId ? locationsList({ site_id: value.siteId }) : Promise.resolve([])
    p.then(setLocations).catch(() => {})
  }, [value.siteId, refresh])

  // Pages are scoped to the chosen site — you only pick/create within it.
  useEffect(() => {
    const p = value.siteId ? pagesList(value.siteId) : Promise.resolve([])
    p.then(setPages).catch(() => {})
  }, [value.siteId, refresh])

  const createSite = () => {
    const nm = newSite.trim()
    if (!nm || creatingSite) return
    const id = mintId('site')
    setCreatingSite(true)
    siteCreate({ site_id: id, name: nm })
      // Refetch the authoritative list FIRST so the new option exists before we
      // select it (a controlled <select> won't hold a value not in its options).
      .then(() => sitesList().catch(() => [{ site_id: id, name: nm } as SiteRow]))
      .then((list) => {
        setSites(list.some((s) => s.site_id === id) ? list : [...list, { site_id: id, name: nm } as SiteRow])
        setNewSite('')
        onChange({ ...EMPTY_PLACEMENT, siteId: id })
      })
      .finally(() => setCreatingSite(false))
  }

  return (
    <div className="flex flex-col gap-4">
      <Field label="Site">
        <Picker
          value={value.siteId}
          placeholder={sites.length ? '+ Choose or create a site' : '+ Create your first site'}
          options={sites.map((s) => ({ value: s.site_id, label: s.name }))}
          onChange={(v) => onChange({ ...EMPTY_PLACEMENT, siteId: v })}
        />
        {!value.siteId && (
          <div className="mt-2 flex gap-2">
            <TextInput
              value={newSite}
              onChange={setNewSite}
              onEnter={createSite}
              placeholder="New site name (e.g. Building A)"
              ariaLabel="New site name"
            />
            <motion.button
              whileTap={{ scale: 0.94 }}
              type="button"
              onClick={createSite}
              disabled={!newSite.trim() || creatingSite}
              aria-label="Create site"
              style={{ backgroundColor: look.accent }}
              className="grid w-12 shrink-0 cursor-pointer place-items-center rounded-xl text-primary-on disabled:opacity-40"
            >
              {creatingSite ? <Loader2 className="h-5 w-5 animate-spin" /> : <Plus className="h-5 w-5" />}
            </motion.button>
          </div>
        )}
      </Field>

      {value.siteId && (
        <Field label="Location">
          <Picker
            value={value.locationId}
            placeholder="+ New location"
            options={locations.map((l) => ({ value: l.location_id, label: l.name }))}
            onChange={(v) => set({ locationId: v, newLocation: '' })}
          />
          {!value.locationId && (
            <TextInput
              value={value.newLocation}
              onChange={(v) => set({ newLocation: v })}
              placeholder="New location name (e.g. Level 3 — North)"
            />
          )}
        </Field>
      )}

      {value.siteId && (
        <Field label="Dashboard page">
          <Picker
            value={value.pageId}
            placeholder="+ New page for this site"
            options={pages.map((p) => ({ value: p.page_id, label: p.name }))}
            onChange={(v) => set({ pageId: v, newPage: '' })}
          />
          {!value.pageId && (
            <TextInput
              value={value.newPage}
              onChange={(v) => set({ newPage: v })}
              placeholder="New page name (e.g. Floor 3 dashboard)"
            />
          )}
        </Field>
      )}
    </div>
  )
}
