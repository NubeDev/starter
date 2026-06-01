import type { ReactNode } from 'react'
import { AnimatePresence, motion } from 'framer-motion'

// The dominant modal recipe, extracted from the design system's QuickAdd sheet.
// Scrim + spring-up glass panel with a grab handle. One sheet per call site.
export function BottomSheet({
  open,
  onClose,
  title,
  children,
}: {
  open: boolean
  onClose: () => void
  title?: string
  children: ReactNode
}) {
  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="absolute inset-0 z-50 bg-black/50 backdrop-blur-sm"
          />
          <motion.div
            initial={{ y: '100%' }}
            animate={{ y: 0 }}
            exit={{ y: '100%' }}
            transition={{ type: 'spring', stiffness: 320, damping: 34 }}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            className="glass-strong absolute inset-x-0 bottom-0 z-50 max-h-[88%] overflow-y-auto rounded-t-[2rem] px-5 pb-8 pt-5"
          >
            {/* grab handle */}
            <div className="mx-auto mb-4 h-1.5 w-10 rounded-full bg-white/20" />
            {title && <p className="label mb-3">{title}</p>}
            {children}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  )
}
