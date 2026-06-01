import { useCallback, useRef, useState, type ReactNode } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { ToastContext } from './toastContext'

// Lightweight toast: one at a time, auto-dismisses. Floats above the nav.
// Adapted from the design system; the context+hook live in toastContext.ts.
interface ToastData {
  id: number
  text: string
  accent: string
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toast, setToast] = useState<ToastData | null>(null)
  const counter = useRef(0)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  const show = useCallback((text: string, accent = '#36e2c4') => {
    counter.current += 1
    setToast({ id: counter.current, text, accent })
    if (timer.current) clearTimeout(timer.current)
    timer.current = setTimeout(() => setToast(null), 3400)
  }, [])

  return (
    <ToastContext.Provider value={{ show }}>
      {children}
      <div className="pointer-events-none absolute inset-x-0 bottom-28 z-[60] flex justify-center px-margin">
        <AnimatePresence>
          {toast && (
            <motion.div
              key={toast.id}
              initial={{ opacity: 0, y: 24, scale: 0.96 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 12, scale: 0.98 }}
              transition={{ type: 'spring', stiffness: 320, damping: 26 }}
              className="glass-strong max-w-full rounded-full px-5 py-3 text-center text-sm font-semibold text-ink"
              style={{ boxShadow: `0 10px 36px -10px ${toast.accent}` }}
              role="status"
              aria-live="polite"
            >
              {toast.text}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </ToastContext.Provider>
  )
}
