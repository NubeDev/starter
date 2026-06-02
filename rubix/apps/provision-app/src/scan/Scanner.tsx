import { useEffect, useRef, useState } from 'react'
import { motion } from 'framer-motion'
import { BrowserMultiFormatReader } from '@zxing/browser'
import { Camera, CameraOff, Keyboard } from 'lucide-react'
import { useLook } from '../theme/useLook'
import { TextInput } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'

// Turn a getUserMedia failure into something a field tech can act on. The
// DOMException `name` is the reliable signal (messages vary by WebView); a
// denied prompt throws NotAllowedError, which on Android means the OS-level
// CAMERA grant was declined and must be re-enabled in app settings.
function describeCamError(e: unknown): string {
  const name = e instanceof DOMException ? e.name : ''
  switch (name) {
    case 'NotAllowedError':
    case 'SecurityError':
      return 'Camera permission denied. Enable it in Settings → Apps → Rubix Provision → Permissions, then reopen the scanner.'
    case 'NotFoundError':
    case 'OverconstrainedError':
      return 'No camera found on this device.'
    case 'NotReadableError':
      return 'Camera is in use by another app. Close it and try again.'
    default:
      return e instanceof Error ? e.message : 'Camera unavailable'
  }
}

// Camera viewport (glass-framed, animated scan-line) + a manual fallback to
// paste/wedge a rubix://add?… URL or serial. Uses @zxing/browser for QR/Code128.
// Emits the raw decoded string upward; the flow decodes it via bc_decode.
export function Scanner({ onCode }: { onCode: (raw: string) => void }) {
  const look = useLook()
  const videoRef = useRef<HTMLVideoElement>(null)
  const [camError, setCamError] = useState<string | null>(null)
  const [manual, setManual] = useState('')

  useEffect(() => {
    // StrictMode (dev) mounts effects twice: mount → cleanup → mount. The
    // camera stream starts asynchronously, so its `controls` resolve AFTER the
    // first cleanup runs — the `if (stopped) c.stop()` below releases a stream
    // that finishes opening post-teardown, so a second mount can't end up with
    // two readers fighting over one <video> element.
    const reader = new BrowserMultiFormatReader()
    let stopped = false
    let controls: { stop: () => void } | null = null

    const stop = () => {
      stopped = true
      controls?.stop()
      controls = null
    }

    // After the stream is live, nudge the rear camera to keep refocusing on a
    // close-held QR label. This is best-effort and additive — it never changes
    // how the camera STARTS (that's decodeFromVideoDevice below, the path that
    // works), only re-asserts continuous autofocus on the already-running track
    // when the hardware reports it as supported. `focusMode` isn't in the
    // standard MediaTrackConstraints type but Android Chromium honours it.
    const enableAutofocus = () => {
      const stream = videoRef.current?.srcObject
      if (!(stream instanceof MediaStream)) return
      const track = stream.getVideoTracks()[0]
      if (!track?.getCapabilities) return
      const caps = track.getCapabilities() as Record<string, unknown>
      if (Array.isArray(caps.focusMode) && caps.focusMode.includes('continuous')) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        track.applyConstraints({ advanced: [{ focusMode: 'continuous' } as any] }).catch(() => {})
      }
    }

    reader
      .decodeFromVideoDevice(undefined, videoRef.current ?? undefined, (result) => {
        if (result && !stopped) {
          onCode(result.getText())
          stop()
        }
      })
      .then((c) => {
        controls = c
        if (stopped) {
          // Torn down before the camera finished opening — release immediately.
          c.stop()
          return
        }
        enableAutofocus()
      })
      .catch((e: unknown) => {
        if (!stopped) setCamError(describeCamError(e))
      })

    return stop
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
