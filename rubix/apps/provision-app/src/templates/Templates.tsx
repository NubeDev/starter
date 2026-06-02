import { useEffect, useState } from 'react'
import { motion } from 'framer-motion'
import { FileCode2, ChevronRight, Loader2, Save, QrCode, Wand2 } from 'lucide-react'
import { useRefreshKey } from '../api/refresh'
import { templatesList, templateYaml, templateUpsert } from '../api/bc'
import { mintId } from '../api/ids'
import { buildAddUrl, addressLabel } from '../scan/buildAddUrl'
import { QrLabel } from '../scan/QrLabel'
import { PageHeader, GlassCard, PrimaryButton } from '../components/ui'
import { BottomSheet } from '../components/BottomSheet'
import { Field, TextInput } from '../components/FormKit'
import { useToast } from '../components/toastContext'
import { useLook } from '../theme/useLook'
import type { TemplateRow } from '../api/bc-types'

// List YAML templates; tap to view/edit the YAML and bc_template_upsert.
export function Templates() {
  const look = useLook()
  const toast = useToast()
  const refresh = useRefreshKey()
  const [rows, setRows] = useState<ReadonlyArray<TemplateRow>>([])
  const [editing, setEditing] = useState<string | null>(null)
  const [yaml, setYaml] = useState('')
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  // QR-generator sheet state: which template, and the ID/address the user types.
  const [qrFor, setQrFor] = useState<TemplateRow | null>(null)
  const [qrId, setQrId] = useState('')
  const [qrAddr, setQrAddr] = useState('')

  useEffect(() => {
    templatesList().then(setRows).catch(() => {})
  }, [refresh])

  const openQr = (t: TemplateRow) => {
    setQrFor(t)
    setQrId('')
    setQrAddr('')
  }

  const mintFor = (t: TemplateRow) =>
    setQrId(mintId(t.template.slice(0, 3).toUpperCase()).replace(/_/g, '-'))

  const open = (t: TemplateRow) => {
    setEditing(t.template)
    setYaml('')
    setLoading(true)
    templateYaml(t.template)
      .then((res) => setYaml(res[0]?.yaml ?? ''))
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Load failed', '#ff5a52'))
      .finally(() => setLoading(false))
  }

  const save = () => {
    setSaving(true)
    templateUpsert(yaml)
      .then(() => {
        toast.show('Template saved', look.accent)
        setEditing(null)
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Save failed', '#ff5a52'))
      .finally(() => setSaving(false))
  }

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <PageHeader eyebrow="Catalog" title="Templates" />

      <div className="flex flex-col gap-2">
        {rows.map((t, i) => (
          <motion.div
            key={t.template}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.04, type: 'spring', stiffness: 300, damping: 30 }}
          >
            <GlassCard onClick={() => open(t)} className="flex items-center gap-3 p-4">
              <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-white/[0.06]">
                <FileCode2 className="h-5 w-5 text-ink-variant" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate font-semibold text-ink">{t.display_name}</p>
                <p className="truncate text-xs text-ink-muted">
                  {t.category} · {t.network} · v{t.version}
                </p>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  openQr(t)
                }}
                aria-label={`Make a QR sticker for ${t.display_name}`}
                className="grid h-9 w-9 shrink-0 cursor-pointer place-items-center rounded-full bg-white/[0.06] text-ink-variant"
              >
                <QrCode className="h-5 w-5" style={{ color: look.accent }} />
              </button>
              <ChevronRight className="h-5 w-5 text-ink-muted" />
            </GlassCard>
          </motion.div>
        ))}
      </div>

      <BottomSheet open={editing !== null} onClose={() => setEditing(null)} title={`Edit · ${editing ?? ''}`}>
        {loading ? (
          <p className="flex items-center gap-2 py-8 text-sm text-ink-muted">
            <Loader2 className="h-4 w-4 animate-spin" /> Loading YAML…
          </p>
        ) : (
          <div className="flex flex-col gap-4">
            <textarea
              value={yaml}
              onChange={(e) => setYaml(e.target.value)}
              spellCheck={false}
              aria-label="Template YAML"
              className="glass h-64 w-full resize-none rounded-xl p-3 font-mono text-xs text-ink outline-none focus:ring-2 focus:ring-primary/60"
            />
            <PrimaryButton accent={look.accent} disabled={saving || !yaml.trim()} onClick={save}>
              {saving ? (
                <span className="inline-flex items-center justify-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" /> Saving…
                </span>
              ) : (
                <span className="inline-flex items-center justify-center gap-2">
                  <Save className="h-5 w-5" /> Save template
                </span>
              )}
            </PrimaryButton>
          </div>
        )}
      </BottomSheet>

      <BottomSheet open={qrFor !== null} onClose={() => setQrFor(null)} title={`QR sticker · ${qrFor?.display_name ?? ''}`}>
        {qrFor && (
          <div className="flex flex-col gap-4">
            <Field label="Device ID / serial">
              <div className="flex gap-2">
                <div className="flex-1">
                  <TextInput
                    value={qrId}
                    onChange={setQrId}
                    placeholder={`${qrFor.template.slice(0, 3).toUpperCase()}-0001`}
                    ariaLabel="Device ID"
                  />
                </div>
                <button
                  onClick={() => mintFor(qrFor)}
                  aria-label="Mint an ID"
                  className="glass grid w-12 shrink-0 place-items-center rounded-xl text-ink-variant"
                >
                  <Wand2 className="h-5 w-5" style={{ color: look.accent }} />
                </button>
              </div>
            </Field>

            <Field label={`${addressLabel(qrFor.network)} (optional)`}>
              <TextInput
                value={qrAddr}
                onChange={setQrAddr}
                placeholder={qrFor.network === 'lora' ? '70B3D5499F2C18' : '192.168.15.42'}
                ariaLabel={addressLabel(qrFor.network)}
              />
            </Field>

            {qrId.trim() ? (
              <QrLabel
                value={buildAddUrl({
                  id: qrId.trim(),
                  model: qrFor.template,
                  network: qrFor.network,
                  address: qrAddr,
                })}
                title={qrFor.display_name}
                subtitle={qrId.trim()}
                caption={buildAddUrl({
                  id: qrId.trim(),
                  model: qrFor.template,
                  network: qrFor.network,
                  address: qrAddr,
                })}
              />
            ) : (
              <p className="flex items-center gap-2 py-6 text-sm text-ink-muted">
                <QrCode className="h-4 w-4" /> Enter or mint an ID to generate the QR.
              </p>
            )}
          </div>
        )}
      </BottomSheet>
    </div>
  )
}
