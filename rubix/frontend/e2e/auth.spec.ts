// Auth happy-path e2e — login → app shell → logout → login slot re-appears.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator `op@example.com` / `rubix-dev-passwd` already
// seeded. The canonical way to bring that up locally is:
//
//     mani run demo
//
// (from the `rubix/` directory — see `rubix/mani.yaml`'s `demo`
// task). The dev Vite server proxies `/api/v1` to 127.0.0.1:8088, so
// the test interacts with the agent through the same origin as the
// browser.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

test.describe('auth', () => {
  test.beforeEach(async ({ context }) => {
    // Start every spec from a clean cookie jar so the AuthProvider
    // renders the unauthenticated slot, not a cached session.
    await context.clearCookies()
  })

  test('login → dashboard renders → logout → login route shows again', async ({
    page,
    request,
  }) => {
    await page.goto('/')

    // Unauthenticated slot from <AuthProvider>: login form is shown.
    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })

    await page.locator('#login-email').fill(EMAIL)
    await page.locator('#login-password').fill(PASSWORD)
    await page.getByRole('button', { name: /^Sign in$/ }).click()

    // After a successful login the AuthProvider swaps the slot for the
    // routed app. We land on `/` which renders the Landing page — the
    // "Sign in to Rubix" card must be gone.
    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({ timeout: 10_000 })
    await expect(page).toHaveURL(/\/(?:\?.*)?$/)

    // The session cookie set by rubix-agent must have landed against
    // the SPA origin — if this is missing every subsequent /api/v1/*
    // call will 401 even though the UI thinks the user is logged in.
    // This is the assertion that catches the prior path-mismatch bug
    // where login() POSTed to /auth/login (no /api/v1) and the vite
    // proxy served the SPA index instead of forwarding to the agent.
    const cookies = await page.context().cookies()
    expect(cookies.map((c) => c.name)).toEqual(
      expect.arrayContaining([expect.stringMatching(/session/i)]),
    )

    // Drive a real authenticated route through the proxy so an
    // accidental "200 HTML from SPA fallback" can no longer pass as
    // a successful me() — /extensions calls /api/v1/extensions and
    // the heading only renders when the call comes back authed.
    await page.goto('/extensions')
    await expect(
      page.getByRole('heading', { name: /Installed extensions/i }),
    ).toBeVisible({ timeout: 10_000 })
    await expect(
      page.getByRole('heading', { name: /Sign in to Rubix/i }),
    ).toBeHidden()

    // Logout via the wire-level endpoint — the UI item in
    // <NavUser> is not yet wired to `auth.logout()` (see
    // src/components/layout/nav-user.tsx). The behaviour we care about
    // here is that the AuthProvider re-renders its unauthenticated
    // slot after the session cookie clears, regardless of which
    // surface triggered the logout.
    const csrf = (await page.context().cookies()).find((c) => /csrf/i.test(c.name))?.value
    const logout = await request.post('http://127.0.0.1:8088/api/v1/auth/logout', {
      headers: csrf ? { 'x-csrf-token': csrf } : undefined,
    })
    expect(logout.ok()).toBeTruthy()
    await page.context().clearCookies()

    await page.goto('/')
    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
  })
})
