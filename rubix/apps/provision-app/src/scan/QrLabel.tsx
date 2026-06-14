import { useRef } from 'react'
import { QRCodeSVG } from 'qrcode.react'
import { Printer } from 'lucide-react'
import { useLook } from '../theme/useLook'
import { useToast } from '../components/toastContext'

// A scannable QR sticker + a "Print" button. `value` is the QR *payload*
// (a `rubix://add?…` string), NOT an image URL — encode it so the printed
// sticker scans back through bc_decode. Shared by the device label sheet and
// the Templates QR generator so every sticker looks and prints identically.
export function QrLabel({
  value,
  title,
  subtitle,
  caption,
}: {
  value: string
  title?: string
  subtitle?: string
  caption?: string
}) {
  const look = useLook()
  const toast = useToast()
  const ref = useRef<HTMLDivElement>(null)

  // Pop a print window with just the label markup so it can go straight to a
  // label printer. We clone the rendered node so the print is pixel-identical.
  const print = () => {
    const node = ref.current
    if (!node) return
    const win = window.open('', '_blank', 'width=420,height=560')
    if (!win) {
      toast.show('Allow pop-ups to print labels', '#ff5a52')
      return
    }
    win.document.write(
      `<!doctype html><html><head><title>${subtitle ?? title ?? 'label'}</title>` +
        `<style>body{margin:0;display:grid;place-items:center;min-height:100vh;` +
        `font-family:ui-sans-serif,system-ui,sans-serif}` +
        `.label{display:flex;flex-direction:column;align-items:center;gap:8px;padding:16px}` +
        `.serial{font-family:ui-monospace,monospace;font-size:12px}` +
        `.name{font-weight:700;font-size:16px}</style></head>` +
        `<body><div class="label">${node.innerHTML}</div>` +
        `<script>window.onload=function(){window.print();window.close()}</script></body></html>`,
    )
    win.document.close()
  }

  return (
    <div className="flex flex-col items-center gap-4 text-center">
      <div ref={ref} className="flex flex-col items-center gap-2">
        <div className="rounded-xl bg-white p-3">
          <QRCodeSVG value={value} size={160} level="M" marginSize={0} title={`QR code for ${subtitle ?? title ?? value}`} />
        </div>
        {title && <p className="name text-lg font-bold text-ink">{title}</p>}
        {subtitle && <p className="serial font-mono text-sm text-ink-variant">{subtitle}</p>}
        {caption && <p className="font-mono text-xs text-ink-muted">{caption}</p>}
      </div>
      <button
        onClick={print}
        className="glass flex w-full cursor-pointer items-center justify-center gap-2 rounded-xl py-3 text-sm font-semibold text-ink"
      >
        <Printer className="h-4 w-4" style={{ color: look.accent }} />
        Print sticker
      </button>
    </div>
  )
}
