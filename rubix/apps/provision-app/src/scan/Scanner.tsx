import { useEffect, useRef, useState } from 'react'
import { motion } from 'framer-motion'
import { BrowserMultiFormatReader } from '@zxing/browser'
import { Camera, CameraOff, Keyboard } from 'lucide-react'
import { useLook } from '../theme/useLook'
import { TextInput } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'

// Camera viewport (glass-framed, animated scan-line) + a manual fallback to
// paste/wedge a rubix://add?… URL or serial. Uses @zxing/browser for QR/Code128.
// Emits the raw decoded string upward; the flow decodes it via bc_decode.
export function Scanner({ onCode }: { onCode: (raw: string) => void }) {
  const look = useLook()
  const videoRef = useRef<HTMLVideoElement>(null)
  const [camError, setCamError] = useState<string | null>(null)
  const [manual, setManual] = useState('')

  useEffect(() => {
    const reader = new BrowserMultiFormatReader()
    let stopped = false
    let controls: { stop: () => void } | null = null

    reader
      .decodeFromVideoDevice(undefined, videoRef.current ?? undefined, (result) => {
        if (result && !stopped) {
          stopped = true
          controls?.stop()
          onCode(result.getText())
        }
      })
      .then((c) => {
        controls = c
        if (stopped) c.stop()
      })
      .catch((e: unknown) => setCamError(e instanceof Error ? e.message : 'Camera unavailable'))

    return () => {
      stopped = true
      controls?.stop()
    }
  }, [onCode])

  return (
    <div className="flex flex-col gap-5">
      {/* camera viewport */}
      <div
        className="glass relative aspect-square w-full overflow-hidden rounded-2xl"
        style={{ boxShadow: `0 0 0 1px ${look.accent}33, 0 18px 50px -20px ${look.accent}` }}
      >
        {camError ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center text-ink-muted">
            <CameraOff className="h-8 w-8" />
            <p className="text-sm">{camError}</p>
            <p className="text-xs">Use manual entry below.</p>
          </div>
        ) : (
          <>
            <video ref={videoRef} className="h-full w-full object-cover" muted playsInline />
            {/* sweeping scan line */}
            <motion.div
              className="pointer-events-none absolute inset-x-6 h-0.5 rounded-full"
              style={{ backgroundColor: look.accent, boxShadow: `0 0 12px 2px ${look.accent}`, animation: 'var(--animate-scanline)' }}
            />
            {/* corner brackets */}
            <div className="pointer-events-none absolute inset-5 rounded-xl border-2 border-white/30" />
            <div className="absolute left-3 top-3 flex items-center gap-1.5 rounded-full bg-black/40 px-2.5 py-1 text-xs font-semibold text-ink">
              <Camera className="h-3.5 w-3.5" style={{ color: look.accent }} />
              Scanning
            </div>
          </>
        )}
      </div>

      {/* manual / wedge entry */}
      <div className="glass rounded-2xl p-4">
        <p className="mb-2 flex items-center gap-2 text-ink-muted">
          <Keyboard className="h-4 w-4" />
          <span className="label">Or paste / scan a code</span>
        </p>
        <div className="flex flex-col gap-3">
          <TextInput
            value={manual}
            onChange={setManual}
            onEnter={() => manual.trim() && onCode(manual.trim())}
            placeholder="rubix://add?id=DRP-9F2C18&model=droplet…"
            ariaLabel="Barcode payload"
          />
          <PrimaryButton accent={look.accent} disabled={!manual.trim()} onClick={() => onCode(manual.trim())}>
            Identify device
          </PrimaryButton>
        </div>
      </div>
    </div>
  )
}
