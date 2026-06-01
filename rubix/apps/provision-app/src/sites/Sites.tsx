import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { Building2, MapPin, Plus, Loader2 } from 'lucide-react'
import { useRefreshKey } from '../api/refresh'
import { sitesList, locationsList, siteCreate, locationCreate } from '../api/bc'
import { mintId } from '../api/ids'
import { PageHeader, GlassCard } from '../components/ui'
import { TextInput } from '../components/FormKit'
import { useToast } from '../components/toastContext'
import { useLook } from '../theme/useLook'
import type { LocationRow, SiteRow } from '../api/bc-types'

// Site + location tree with inline create at both levels.
export function Sites() {
  const look = useLook()
  const toast = useToast()
  const refresh = useRefreshKey()
  const [sites, setSites] = useState<ReadonlyArray<SiteRow>>([])
  const [newSite, setNewSite] = useState('')
  const [creating, setCreating] = useState(false)

  useEffect(() => {
    sitesList().then(setSites).catch(() => {})
  }, [refresh])

  const addSite = () => {
    const nm = newSite.trim()
    if (!nm || creating) return
    setCreating(true)
    siteCreate({ site_id: mintId('site'), name: nm })
      .then(() => {
        setNewSite('')
        toast.show('Site created', look.accent)
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Failed', '#ff5a52'))
      .finally(() => setCreating(false))
  }

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <PageHeader eyebrow="Topology" title="Sites" />

      <div className="glass mb-5 flex items-center gap-2 rounded-xl p-2 pl-3">
        <Building2 className="h-4 w-4 text-ink-muted" />
        <div className="flex-1">
          <TextInput value={newSite} onChange={setNewSite} onEnter={addSite} placeholder="New site name" ariaLabel="New site name" />
        </div>
        <motion.button
          whileTap={{ scale: 0.94 }}
          onClick={addSite}
          disabled={!newSite.trim() || creating}
          aria-label="Create site"
          style={{ backgroundColor: look.accent }}
          className="grid h-10 w-10 shrink-0 cursor-pointer place-items-center rounded-lg text-primary-on disabled:opacity-40"
        >
          {creating ? <Loader2 className="h-5 w-5 animate-spin" /> : <Plus className="h-5 w-5" />}
        </motion.button>
      </div>

      <div className="flex flex-col gap-3">
        {sites.map((s) => (
          <SiteNode key={s.site_id} site={s} />
        ))}
      </div>
    </div>
  )
}

function SiteNode({ site }: { site: SiteRow }) {
  const look = useLook()
  const toast = useToast()
  const refresh = useRefreshKey()
  const [locations, setLocations] = useState<ReadonlyArray<LocationRow>>([])
  const [newLoc, setNewLoc] = useState('')

  useEffect(() => {
    locationsList({ site_id: site.site_id }).then(setLocations).catch(() => {})
  }, [site.site_id, refresh])

  const addLoc = () => {
    const nm = newLoc.trim()
    if (!nm) return
    locationCreate({ location_id: mintId('loc'), site_id: site.site_id, name: nm })
      .then(() => {
        setNewLoc('')
        toast.show('Location added', look.accent)
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Failed', '#ff5a52'))
  }

  return (
    <GlassCard className="p-4">
      <div className="flex items-center gap-2">
        <Building2 className="h-5 w-5" style={{ color: look.accent }} />
        <p className="font-bold text-ink">{site.name}</p>
      </div>

      <ul className="mt-3 flex flex-col gap-1.5 pl-2">
        {locations.map((l) => (
          <li key={l.location_id} className="flex items-center gap-2 rounded-lg bg-white/[0.04] px-3 py-2 text-sm text-ink">
            <MapPin className="h-4 w-4 text-ink-muted" />
            {l.name}
          </li>
        ))}
      </ul>

      <div className="mt-2 flex items-center gap-2 pl-2">
        <div className="flex-1">
          <TextInput value={newLoc} onChange={setNewLoc} onEnter={addLoc} placeholder="+ New location" ariaLabel={`New location in ${site.name}`} />
        </div>
        <motion.button
          whileTap={{ scale: 0.94 }}
          onClick={addLoc}
          disabled={!newLoc.trim()}
          aria-label="Add location"
          className="grid h-10 w-10 shrink-0 cursor-pointer place-items-center rounded-lg bg-white/[0.08] text-ink disabled:opacity-40"
        >
          <Plus className="h-5 w-5" />
        </motion.button>
      </div>
    </GlassCard>
  )
}
