import { motion } from 'framer-motion'
import { Cpu, Network, Hash } from 'lucide-react'
import { useLook } from '../theme/useLook'
import type { ScannedIdentity } from '../api/bc-types'

// The decoded device card: model · network · serial, template icon, and a
// preview of the template's points. Shown right after bc_decode.
export function IdentityCard({ identity }: { identity: ScannedIdentity }) {
  const look = useLook()
  const t = identity.template
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ type: 'spring', stiffness: 300, damping: 28 }}
      className="glass rounded-2xl p-5 shadow-glass"
    >
      <div className="flex items-center gap-3">
        <div
          className="grid h-12 w-12 shrink-0 place-items-center rounded-xl"
          style={{ backgroundColor: `${look.accent}22` }}
        >
          <Cpu className="h-6 w-6" style={{ color: look.accent }} />
        </div>
        <div className="min-w-0">
          <p className="truncate text-lg font-bold text-ink">{t.display_name}</p>
          <p className="truncate text-sm text-ink-variant">{identity.model}</p>
        </div>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Meta icon={Network} text={identity.network} />
        <Meta icon={Hash} text={identity.id} />
      </div>

      <p className="label mt-5 mb-2">Points · {t.points.length}</p>
      <ul className="flex flex-col gap-1.5">
        {t.points.map((p) => (
          <li key={p.key} className="flex items-center justify-between rounded-lg bg-white/[0.04] px-3 py-2">
            <span className="text-sm text-ink">{p.name}</span>
            <span className="label">{p.widget}</span>
          </li>
        ))}
      </ul>
    </motion.div>
  )
}

function Meta({ icon: Icon, text }: { icon: typeof Cpu; text: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full bg-white/[0.06] px-3 py-1.5 text-xs font-semibold text-ink-variant">
      <Icon className="h-3.5 w-3.5" />
      {text}
    </span>
  )
}
