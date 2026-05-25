// `LoginRoute` — email/password form rendered by `<AuthProvider>` as
// its `unauthenticatedSlot` whenever `me()` returns 401. Acting as a
// slot rather than a routed page means the user keeps their target
// URL in the address bar; on success we read `?returnTo=` (if
// present) and navigate there, otherwise we land on `/`.
//
// A `createFileRoute('/login')` is also exported so a direct hit on
// `/login` (e.g. external link, redirect) still renders the same
// form — the TanStack Router file-routes plugin requires every file
// under `src/routes/` to export a `Route`, so we couple them.

import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useState, type FormEvent } from 'react'
import { useAuth } from '@nube/starter-client-react'
import { StarterError } from '@nube/starter-client-ts'
import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from '@nube/starter-ui-kit'

export function LoginRoute() {
  const auth = useAuth()
  const router = useRouter()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    setError(null)
    setSubmitting(true)
    try {
      await auth.login({ email, password })
      // Honour `?returnTo=` if present — otherwise land on `/`. We
      // intentionally restrict to same-origin paths so a crafted URL
      // can't bounce the user to an external login-stealer.
      const params = new URLSearchParams(window.location.search)
      const returnTo = params.get('returnTo')
      const target = returnTo && returnTo.startsWith('/') ? returnTo : '/'
      await router.navigate({ to: target })
    } catch (err) {
      const msg =
        err instanceof StarterError
          ? err.status === 401
            ? 'Invalid email or password.'
            : `Login failed (${err.status}).`
          : 'Login failed. Please try again.'
      setError(msg)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-[color:var(--color-bg)] p-6">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>
            <h2 className="m-0 p-0 text-inherit">Sign in to Rubix</h2>
          </CardTitle>
          <CardDescription>
            Use your operator credentials to continue.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor="login-email">Email</Label>
              <Input
                id="login-email"
                type="email"
                autoComplete="username"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={submitting}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="login-password">Password</Label>
              <Input
                id="login-password"
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                disabled={submitting}
              />
            </div>
            {error ? (
              <p role="alert" className="text-sm text-[color:var(--color-danger,#dc2626)]">
                {error}
              </p>
            ) : null}
            <Button type="submit" disabled={submitting}>
              {submitting ? 'Signing in…' : 'Sign in'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}

export const Route = createFileRoute('/login')({ component: LoginRoute })
