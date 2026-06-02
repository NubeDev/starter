import { useCallback, useState } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { ArrowLeft, Loader2, ScanLine, Info } from 'lucide-react'
import { Scanner } from './Scanner'
import { TypePicker } from './TypePicker'
import { IdentityCard } from '../identify/IdentityCard'
import { Place } from '../place/Place'
import { EMPTY_PLACEMENT, placementReady, placementHasPage, type Placement } from '../place/placement'
import { TogglesStep } from '../provision/TogglesStep'
import { ProvisionedReveal } from '../provision/ProvisionedReveal'
import { buildProvisionInput } from './buildProvisionInput'
import { decode, provision } from '../api/bc'
import { useLook } from '../theme/useLook'
import { useToast } from '../components/toastContext'
import { PageHeader, PrimaryButton, SectionLabel } from '../components/ui'
import { Chip } from '../components/ui'
import { Field, TextInput } from '../components/FormKit'
import type { ProvisionResult, ScannedIdentity } from '../api/bc-types'

type Step = 'scan' | 'identify' | 'place' | 'confirm' | 'done'

// The phone flow controller: Scan → Identify → Place → Toggles → Confirm.
// Each step is a screen; one bc_provision call lands the device and pops the
// success reveal. `onPreview` jumps the shell to the Page preview tab.
export function ScanFlow({ onPreview }: { onPreview: (pageId: string) => void }) {
  const look = useLook()
  const toast = useToast()
  const [step, setStep] = useState<Step>('scan')
  const [mode, setMode] = useState<'scan' | 'type'>('scan')
  const [barcode, setBarcode] = useState('')
  const [identity, setIdentity] = useState<ScannedIdentity | null>(null)
  const [place, setPlace] = useState<Placement>(EMPTY_PLACEMENT)
  const [name, setName] = useState('')
  const [trend, setTrend] = useState(true)
  const [alarm, setAlarm] = useState(true)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<ProvisionResult | null>(null)

  const reset = useCallback(() => {
    setStep('scan')
    setMode('scan')
    setBarcode('')
    setIdentity(null)
    setPlace(EMPTY_PLACEMENT)
    setName('')
    setTrend(true)
    setAlarm(true)
    setResult(null)
  }, [])

  const onCode = useCallback(
    (raw: string) => {
      setBarcode(raw)
      setBusy(true)
      decode(raw)
        .then((id) => {
          setIdentity(id)
          setStep('identify')
        })
        .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Could not decode', '#ff5a52'))
        .finally(() => setBusy(false))
    },
    [toast],
  )

  const confirm = useCallback(() => {
    setBusy(true)
    provision(buildProvisionInput(barcode, place, trend, alarm, name.trim() || undefined))
      .then((r) => {
        setResult(r)
        setStep('done')
      })
      .catch((e: unknown) => toast.show(e instanceof Error ? e.message : 'Provision failed', '#ff5a52'))
      .finally(() => setBusy(false))
  }, [barcode, place, trend, alarm, name, toast])

  return (
    <div className="h-full overflow-y-auto px-margin pb-32 pt-20 sm:pt-24">
      <div className="mb-2 flex items-center gap-2">
        {step !== 'scan' && step !== 'done' && (
          <button
            onClick={() => setStep(step === 'confirm' ? 'place' : step === 'place' ? 'identify' : 'scan')}
            aria-label="Back"
            className="grid h-9 w-9 cursor-pointer place-items-center rounded-full bg-white/[0.06] text-ink-variant"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <PageHeader eyebrow={STEP_LABEL[step]} title={STEP_TITLE[step]} />
      </div>

      <AnimatePresence mode="wait">
        <motion.div
          key={step}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          transition={{ duration: 0.2 }}
        >
          {step === 'scan' && (
            <div className="flex flex-col gap-4">
              <div className="flex gap-2">
                <Chip label="Scan barcode" active={mode === 'scan'} onClick={() => setMode('scan')} />
                <Chip label="Pick a type" active={mode === 'type'} onClick={() => setMode('type')} />
              </div>
              {busy && (
                <p className="flex items-center gap-2 text-sm text-ink-muted">
                  <Loader2 className="h-4 w-4 animate-spin" /> Decoding…
                </p>
              )}
              {mode === 'scan' ? <Scanner onCode={onCode} /> : <TypePicker onSynthesized={onCode} />}
            </div>
          )}

          {step === 'identify' && identity && (
            <div className="flex flex-col gap-5">
              <IdentityCard identity={identity} />
              <PrimaryButton accent={look.accent} onClick={() => setStep('place')}>
                Place this device
              </PrimaryButton>
            </div>
          )}

          {step === 'place' && (
            <div className="flex flex-col gap-5">
              <Place value={place} onChange={setPlace} />
              <PrimaryButton accent={look.accent} disabled={!placementReady(place)} onClick={() => setStep('confirm')}>
                Continue
              </PrimaryButton>
              {!placementReady(place) ? (
                <p className="-mt-2 flex items-center gap-1.5 text-xs text-ink-muted">
                  <Info className="h-3.5 w-3.5" /> Pick a site to continue — a page is optional.
                </p>
              ) : !placementHasPage(place) ? (
                <p className="-mt-2 flex items-center gap-1.5 text-xs text-ink-muted">
                  <Info className="h-3.5 w-3.5" /> No page chosen — it’ll be commissioned as pending. Place it on a page
                  anytime from Devices.
                </p>
              ) : null}
            </div>
          )}

          {step === 'confirm' && (
            <div className="flex flex-col gap-5">
              <Field label="Device name">
                <TextInput
                  value={name}
                  onChange={setName}
                  placeholder={identity?.id ?? 'e.g. L3 North Droplet'}
                />
              </Field>
              <SectionLabel>Behavior</SectionLabel>
              <TogglesStep trend={trend} alarm={alarm} onTrend={setTrend} onAlarm={setAlarm} />
              {!placementHasPage(place) && (
                <p className="flex items-center gap-1.5 text-xs text-ink-muted">
                  <Info className="h-3.5 w-3.5" /> Commissioned as pending — place it on a page anytime from Devices.
                </p>
              )}
              <PrimaryButton accent={look.accent} disabled={busy} onClick={confirm}>
                {busy ? (
                  <span className="inline-flex items-center justify-center gap-2">
                    <Loader2 className="h-5 w-5 animate-spin" /> Provisioning…
                  </span>
                ) : (
                  'Add device'
                )}
              </PrimaryButton>
            </div>
          )}

          {step === 'scan' && (
            <p className="mt-6 flex items-center justify-center gap-2 text-xs text-ink-muted">
              <ScanLine className="h-3.5 w-3.5" /> QR or Code128 · or paste a rubix://add URL
            </p>
          )}
        </motion.div>
      </AnimatePresence>

      <AnimatePresence>
        {step === 'done' && result && (
          <ProvisionedReveal
            result={result}
            onPreview={
              result.page_id
                ? () => {
                    const pid = result.page_id
                    reset()
                    onPreview(pid)
                  }
                : undefined
            }
            onAddAnother={reset}
          />
        )}
      </AnimatePresence>
    </div>
  )
}

const STEP_LABEL: Record<Step, string> = {
  scan: 'Step 1 · Scan',
  identify: 'Step 2 · Identify',
  place: 'Step 3 · Place',
  confirm: 'Step 4 · Confirm',
  done: 'Done',
}
const STEP_TITLE: Record<Step, string> = {
  scan: 'Add a device',
  identify: 'Identify',
  place: 'Where does it go?',
  confirm: 'Trending & alarming',
  done: 'Provisioned',
}
