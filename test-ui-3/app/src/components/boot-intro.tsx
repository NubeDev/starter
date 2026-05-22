import { motion, AnimatePresence } from 'motion/react'
import { useEffect, useState } from 'react'
import { Leaf } from 'lucide-react'

/**
 * Boot intro — plays once on first paint. Apple-grade open.
 *  1. iris reveals the brand mark
 *  2. brand mark expands → fades into the page
 *  3. a green→aqua sheet wipes off downward
 */
export function BootIntro() {
  const [stage, setStage] = useState<'logo' | 'wipe' | 'done'>('logo')

  useEffect(() => {
    const t1 = setTimeout(() => setStage('wipe'), 1500)
    const t2 = setTimeout(() => setStage('done'), 2400)
    return () => {
      clearTimeout(t1)
      clearTimeout(t2)
    }
  }, [])

  return (
    <AnimatePresence>
      {stage !== 'done' && (
        <motion.div
          key="boot"
          className="fixed inset-0 z-[100] flex items-center justify-center overflow-hidden"
          initial={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.4 }}
        >
          {/* base sheet — wipes down */}
          <motion.div
            className="absolute inset-0"
            style={{
              background:
                'radial-gradient(circle at 50% 40%, #0d1a15 0%, #06100c 60%)',
            }}
            initial={{ y: 0 }}
            animate={{ y: stage === 'wipe' ? '-100%' : 0 }}
            transition={{ duration: 0.9, ease: [0.76, 0, 0.24, 1] }}
          />

          {/* accent reveal — green→aqua follows behind */}
          <motion.div
            className="absolute inset-0"
            style={{
              background:
                'linear-gradient(180deg, #4ade80 0%, #22c55e 35%, #06b6d4 100%)',
            }}
            initial={{ y: '100%' }}
            animate={{ y: stage === 'wipe' ? '-100%' : '100%' }}
            transition={{ duration: 1.1, ease: [0.76, 0, 0.24, 1], delay: 0.05 }}
          />

          {/* center mark */}
          <motion.div
            className="relative z-10 flex flex-col items-center gap-4"
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{
              opacity: stage === 'logo' ? 1 : 0,
              scale: stage === 'logo' ? 1 : 1.15,
            }}
            transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
          >
            {/* breathing ring */}
            <div className="relative flex h-24 w-24 items-center justify-center">
              <motion.div
                className="absolute inset-0 rounded-full"
                style={{
                  background:
                    'radial-gradient(circle, rgba(74,222,128,0.45) 0%, transparent 70%)',
                }}
                animate={{ scale: [1, 1.4, 1], opacity: [0.6, 0.2, 0.6] }}
                transition={{ duration: 1.4, repeat: Infinity, ease: 'easeInOut' }}
              />
              <motion.div
                className="relative flex h-16 w-16 items-center justify-center rounded-2xl bg-[color:var(--color-leaf)] text-[color:var(--color-bg)] shadow-[0_0_40px_rgba(74,222,128,0.5)]"
                initial={{ rotate: -90, opacity: 0 }}
                animate={{ rotate: 0, opacity: 1 }}
                transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
              >
                <Leaf className="h-8 w-8" strokeWidth={2.25} />
              </motion.div>
            </div>

            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.3 }}
              className="text-[11px] font-medium uppercase tracking-[0.4em] text-[color:var(--color-mist)]"
            >
              Breathe · Drink · Grow
            </motion.div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
