import { useState } from 'react'
import { motion } from 'framer-motion'
import { ScanLine, Plug, Loader2 } from 'lucide-react'
import { useAuth } from './authContext'
import { useLook } from '../theme/useLook'
import { Field, TextInput } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'

// Gate screen: agent base URL + email/password. Dev defaults baked in as
// placeholders. Shown until authed. Web build needs the base URL; the Tauri
// build still collects it so the Rust core knows which host to reach.
export function Connect() {
  const { login, busy, error, transportKind } = useAuth()
  const look = useLook()
  const [baseUrl, setBaseUrl] = useState('http://127.0.0.1:8088')
  const [email, setEmail] = useState('op@example.com')
  const [password, setPassword] = useState('rubix-dev-passwd')

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
            <TextInput value={baseUrl} onChange={setBaseUrl} type="url" placeholder="http://127.0.0.1:8088" />
          </Field>
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
