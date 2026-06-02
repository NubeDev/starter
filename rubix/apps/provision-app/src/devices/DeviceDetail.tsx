import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { ArrowLeft, MapPin, Pencil, Printer, Trash2 } from 'lucide-react'
import { useRefreshKey } from '../api/refresh'
import { pointsByDevice, deviceUpdate, decommission, labelRender } from '../api/bc'
import { Toggle, TextInput } from '../components/FormKit'
import { GlassCard, SectionLabel } from '../components/ui'
import { BottomSheet } from '../components/BottomSheet'
import { PlaceOnPageSheet } from './PlaceOnPageSheet'
import { useToast } from '../components/toastContext'
import { useLook } from '../theme/useLook'
import { statusColor, isPlaceable } from './statusDot'
import type { DeviceRow, LabelRender, PointRow } from '../api/bc-types'

// Drill-in: a device's points with per-point trend/alarm toggles, inline
// rename, print-label sheet, and decommission.
export function DeviceDetail({ device, onBack }: { device: DeviceRow; onBack: () => void }) {
  const look = useLook()
  const toast = useToast()
  const refresh = useRefreshKey()
  const [points, setPoints] = useState<ReadonlyArray<PointRow>>([])
  const [renaming, setRenaming] = useState(false)
  const [name, setName] = useState(device.name ?? '')
  const [label, setLabel] = useState<LabelRender | null>(null)
  const [placing, setPlacing] = useState(false)

  useEffect(() => {
    pointsByDevice(device.device_id).then(setPoints).catch(() => {})
  }, [device.device_id, refresh])

  const saveName = () => {
    setRenaming(false)
    if (name.trim() === (device.name ?? '')) return
    deviceUpdate({ device_id: device.device_id, name: name.trim() })
      .then(() => toast.show('Renamed', look.accent))
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Rename failed', '#ff5a52'))
  }

  const togglePoint = (p: PointRow, field: 'trend_on' | 'alarm_on') => {
    deviceUpdate({ device_id: device.device_id, point_id: p.point_id, [field]: !p[field] })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Update failed', '#ff5a52'))
  }

  const decom = () => {
    decommission([device.device_id])
      .then(() => {
        toast.show('Decommissioned', '#ffc24b')
        onBack()
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Failed', '#ff5a52'))
  }

  const printLabel = () => {
    labelRender(device.device_id)
      .then(setLabel)
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Label failed', '#ff5a52'))
  }

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <div className="mb-5 flex items-center gap-3">
        <button
          onClick={onBack}
          aria-label="Back to devices"
          className="grid h-9 w-9 cursor-pointer place-items-center rounded-full bg-white/[0.06] text-ink-variant"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <div className="min-w-0 flex-1">
          {renaming ? (
            <TextInput value={name} onChange={setName} onEnter={saveName} ariaLabel="Device name" />
          ) : (
            <button
              onClick={() => setRenaming(true)}
              className="flex items-center gap-2 text-left"
              aria-label="Rename device"
            >
              <span className="truncate text-headline-mobile text-ink">{device.name ?? device.device_id}</span>
              <Pencil className="h-4 w-4 shrink-0 text-ink-muted" />
            </button>
          )}
          <p className="mt-0.5 flex items-center gap-1.5 text-sm text-ink-variant">
            <span className="h-2 w-2 rounded-full" style={{ backgroundColor: statusColor(device.status) }} />
            {device.template} · {device.status}
          </p>
        </div>
      </div>

      <div className="mb-5 flex gap-2">
        {isPlaceable(device) && (
          <Action icon={MapPin} label="Place on page" onClick={() => setPlacing(true)} accent={statusColor('pending')} />
        )}
        <Action icon={Printer} label="Label" onClick={printLabel} accent={look.accent} />
        <Action icon={Trash2} label="Decommission" onClick={decom} accent="#ffc24b" />
      </div>

      <SectionLabel>Points · {points.length}</SectionLabel>
      <div className="flex flex-col gap-2">
        {points.map((p) => (
          <GlassCard key={p.point_id} className="p-4">
            <div className="flex items-center justify-between">
              <div className="min-w-0">
                <p className="truncate font-semibold text-ink">{p.name}</p>
                <p className="text-xs text-ink-muted">
                  {p.widget}
                  {p.unit ? ` · ${p.unit}` : ''}
                </p>
              </div>
              <div className="flex items-center gap-4">
                <ToggleLabeled label="Trend" on={p.trend_on} onToggle={() => togglePoint(p, 'trend_on')} accent={look.accent} />
                <ToggleLabeled label="Alarm" on={p.alarm_on} onToggle={() => togglePoint(p, 'alarm_on')} accent={look.accent} />
              </div>
            </div>
          </GlassCard>
        ))}
      </div>

      <BottomSheet open={!!label} onClose={() => setLabel(null)} title="Print label">
        {label && (
          <div className="flex flex-col items-center gap-3 text-center">
            <img src={label.qr_url} alt={`QR code for ${label.serial}`} className="h-40 w-40 rounded-xl bg-white p-2" />
            <p className="text-lg font-bold text-ink">{label.display_name}</p>
            <p className="font-mono text-sm text-ink-variant">{label.serial}</p>
            <p className="font-mono text-xs text-ink-muted">{label.code128}</p>
          </div>
        )}
      </BottomSheet>

      <PlaceOnPageSheet device={device} open={placing} onClose={() => setPlacing(false)} />
    </div>
  )
}

function Action({
  icon: Icon,
  label,
  onClick,
  accent,
}: {
  icon: typeof Printer
  label: string
  onClick: () => void
  accent: string
}) {
  return (
    <motion.button
      whileTap={{ scale: 0.96 }}
      onClick={onClick}
      className="glass flex flex-1 cursor-pointer items-center justify-center gap-2 rounded-xl py-3 text-sm font-semibold text-ink"
    >
      <Icon className="h-4 w-4" style={{ color: accent }} />
      {label}
    </motion.button>
  )
}

function ToggleLabeled({
  label,
  on,
  onToggle,
  accent,
}: {
  label: string
  on: boolean
  onToggle: () => void
  accent: string
}) {
  return (
    <div className="flex flex-col items-center gap-1">
      <Toggle on={on} onToggle={onToggle} accent={accent} label={label} />
      <span className="label">{label}</span>
    </div>
  )
}
