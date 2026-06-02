import { useEffect, useMemo, useState } from 'react'
import { motion } from 'framer-motion'
import { Search, Cpu, ChevronRight, Inbox, MapPin } from 'lucide-react'
import { useRefreshKey } from '../api/refresh'
import { devicesList } from '../api/bc'
import { PageHeader, GlassCard } from '../components/ui'
import { TextInput } from '../components/FormKit'
import { DeviceDetail } from './DeviceDetail'
import { PlaceOnPageSheet } from './PlaceOnPageSheet'
import { statusColor, isPlaceable } from './statusDot'
import type { DeviceRow } from '../api/bc-types'

// Device list with search; tap a row to drill into points + per-point toggles.
export function Devices() {
  const refresh = useRefreshKey()
  const [rows, setRows] = useState<ReadonlyArray<DeviceRow>>([])
  const [q, setQ] = useState('')
  // Store only the id; derive the row from the live list so a refresh (e.g.
  // after a rename) flows into the open detail without a sync setState.
  const [selectedId, setSelectedId] = useState<string | null>(null)
  // The device whose place-on-page sheet is open (derived from the live list).
  const [placeId, setPlaceId] = useState<string | null>(null)

  useEffect(() => {
    devicesList().then(setRows).catch(() => {})
  }, [refresh])

  const selected = selectedId ? (rows.find((r) => r.device_id === selectedId) ?? null) : null
  const placing = placeId ? (rows.find((r) => r.device_id === placeId) ?? null) : null

  const filtered = useMemo(() => {
    const t = q.trim().toLowerCase()
    if (!t) return rows
    return rows.filter(
      (r) =>
        r.device_id.toLowerCase().includes(t) ||
        (r.name ?? '').toLowerCase().includes(t) ||
        r.template.toLowerCase().includes(t),
    )
  }, [rows, q])

  if (selected) return <DeviceDetail device={selected} onBack={() => setSelectedId(null)} />

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <PageHeader eyebrow="Inventory" title="Devices" />

      <div className="glass mb-5 flex items-center gap-2 rounded-xl px-3">
        <Search className="h-4 w-4 text-ink-muted" />
        <div className="flex-1 py-1">
          <TextInput value={q} onChange={setQ} placeholder="Search devices" ariaLabel="Search devices" />
        </div>
      </div>

      {filtered.length ? (
        <div className="flex flex-col gap-2">
          {filtered.map((d, i) => (
            <motion.div
              key={d.device_id}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.04, type: 'spring', stiffness: 300, damping: 30 }}
            >
              <GlassCard onClick={() => setSelectedId(d.device_id)} className="flex items-center gap-3 p-4">
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-white/[0.06]">
                  <Cpu className="h-5 w-5 text-ink-variant" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate font-semibold text-ink">{d.name ?? d.device_id}</p>
                  <p className="truncate text-xs text-ink-muted">
                    {d.template}
                    {d.network ? ` · ${d.network}` : ''}
                  </p>
                </div>
                {isPlaceable(d) && (
                  <motion.button
                    whileTap={{ scale: 0.94 }}
                    onClick={(e) => {
                      e.stopPropagation()
                      setPlaceId(d.device_id)
                    }}
                    aria-label="Place on page"
                    className="flex shrink-0 items-center gap-1 rounded-full px-2.5 py-1.5 text-xs font-semibold"
                    style={{ backgroundColor: `${statusColor('pending')}22`, color: statusColor('pending') }}
                  >
                    <MapPin className="h-3.5 w-3.5" /> Place
                  </motion.button>
                )}
                <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: statusColor(d.status) }} />
                <ChevronRight className="h-5 w-5 text-ink-muted" />
              </GlassCard>
            </motion.div>
          ))}
        </div>
      ) : (
        <div className="glass mt-4 flex flex-col items-center gap-3 rounded-2xl px-6 py-12 text-center text-ink-muted">
          <Inbox className="h-8 w-8" />
          <p className="max-w-[240px] text-sm">No devices yet. Scan one to get started.</p>
        </div>
      )}

      {placing && (
        <PlaceOnPageSheet device={placing} open={!!placing} onClose={() => setPlaceId(null)} />
      )}
    </div>
  )
}
