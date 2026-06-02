import { useState } from 'react'
import { motion } from 'framer-motion'
import { ScanLine, Plug, Loader2, Wifi, CheckCircle2, XCircle } from 'lucide-react'
import { useAuth } from './authContext'
import { transport } from '../transport'
import { useLook } from '../theme/useLook'
import { Field, TextInput } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'

// Gate screen: agent base URL + email/password. Dev defaults baked in as
// placeholders. Shown until authed. Web build needs the base URL; the Tauri
// build still collects it so the Rust core knows which host to reach.
// Default agent base URL for the Connect form. Not hardcoded to one LAN IP:
// set VITE_AGENT_URL at build/dev time to point the mobile build at your
// machine (e.g. `VITE_AGENT_URL=http://192.168.1.50:8088 cargo tauri android dev`).
// Falls back to localhost, which is correct for the desktop build and for a
// device using `adb reverse tcp:8088 tcp:8088`.
const DEFAULT_AGENT_URL = import.meta.env.VITE_AGENT_URL ?? 'http://127.0.0.1:8088'

export function Connect() {
  const { ping, login, busy, error, transportKind } = useAuth()
  const look = useLook()
  // Seed the host field with the agent this device last connected to (remembered
  // across logout — logout clears credentials, not the host), falling back to the
  // compiled-in default on first run. Read lazily on mount, not at module load,
  // so a logout after a same-session login still re-fills the host that was used
  // instead of reverting to 127.0.0.1.
  const [baseUrl, setBaseUrl] = useState(() => transport.savedBaseUrl?.() || DEFAULT_AGENT_URL)
  const [email, setEmail] = useState('op@example.com')
  const [password, setPassword] = useState('rubix-dev-passwd')

  // Pre-login reachability probe. `null` = not yet pinged.
  const [pinging, setPinging] = useState(false)
  const [pingResult, setPingResult] = useState<{ ok: boolean; message: string } | null>(null)

  const doPing = () => {
    if (pinging) return
    setPinging(true)
    setPingResult(null)
    void ping(baseUrl.trim())
      .then((r) => setPingResult({ ok: r.ok, message: r.message }))
      .catch((e) => setPingResult({ ok: false, message: e instanceof Error ? e.message : String(e) }))
      .finally(() => setPinging(false))
  }

  const submit = () => {
    if (busy) return
    void login(baseUrl.trim(), email.trim(), password).catch(() => {
      /* surfaced via `error` */
    })
  }

  return (
    <div className="flex h-full flex-col justify-center px-margin pb-16 pt-14">
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ type: 'spring', stiffness: 300, damping: 30 }}
        className="mb-8 flex flex-col items-center text-center"
      >
        <div
          className="mb-4 grid h-16 w-16 place-items-center rounded-2xl"
          style={{ backgroundColor: look.accent, boxShadow: `0 12px 40px -10px ${look.accent}` }}
        >
          <ScanLine className="h-8 w-8 text-primary-on" />
        </div>
        <h1 className="text-headline-mobile text-ink">Rubix Provision</h1>
        <p className="mt-1 text-sm text-ink-variant">Scan a device. Place it. Done.</p>
      </motion.div>

      <div className="glass rounded-2xl p-5 shadow-glass">
        <div className="mb-2 flex items-center gap-2 text-ink-muted">
          <Plug className="h-4 w-4" />
          <span className="label">
            {transportKind === 'tauri' ? 'Native · Tauri core' : 'Browser · direct REST'}
          </span>
        </div>
        <div className="flex flex-col gap-4">
          <Field label="Agent base URL">
            <TextInput
              value={baseUrl}
              onChange={(v) => {
                setBaseUrl(v)
                setPingResult(null) // stale verdict no longer applies to the new host
              }}
              type="url"
              placeholder="http://127.0.0.1:8088"
            />
          </Field>

          <button
            type="button"
            onClick={doPing}
            disabled={pinging}
            className="inline-flex items-center justify-center gap-2 rounded-xl border border-white/15 px-4 py-2 text-sm font-medium text-ink transition disabled:opacity-60"
          >
            {pinging ? <Loader2 className="h-4 w-4 animate-spin" /> : <Wifi className="h-4 w-4" />}
            {pinging ? 'Pinging…' : 'Ping agent'}
          </button>

          {pingResult && (
            <p
              className={`inline-flex items-center gap-2 text-sm font-medium ${
                pingResult.ok ? 'text-online' : 'text-fault'
              }`}
            >
              {pingResult.ok ? (
                <CheckCircle2 className="h-4 w-4 shrink-0" />
              ) : (
                <XCircle className="h-4 w-4 shrink-0" />
              )}
              <span className="break-all">{pingResult.message}</span>
            </p>
          )}
          <Field label="Email">
            <TextInput value={email} onChange={setEmail} type="email" placeholder="op@example.com" />
          </Field>
          <Field label="Password">
            <TextInput
              value={password}
              onChange={setPassword}
              type="password"
              placeholder="••••••••"
              onEnter={submit}
            />
          </Field>

          {error && (
            <p role="alert" className="text-sm font-medium text-fault">
              {error}
            </p>
          )}

          <PrimaryButton onClick={submit} accent={look.accent} disabled={busy}>
            {busy ? (
              <span className="inline-flex items-center justify-center gap-2">
                <Loader2 className="h-5 w-5 animate-spin" /> Connecting…
              </span>
            ) : (
              'Connect'
            )}
          </PrimaryButton>
        </div>
      </div>
    </div>
  )
}
