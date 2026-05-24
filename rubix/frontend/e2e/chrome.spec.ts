// Chrome smoke test — exercises the polished app chrome landed in
// Phase D: login → top-header surfaces the operator's email and a
// working logout → the left nav lists the five top-level sections
// (Home, Flows, Extensions, Admin, Settings) → logout drops back to
// the AuthProvider's unauthenticated slot (the "/login" surface,
// rendered via the slot rather than a dedicated route — see
// SCOPE OQ-4 in src/main.tsx).
//
// Prereq mirrors auth.spec.ts: a running rubix-agent on :8088 with
// the bootstrap operator op@example.com / rubix-dev-passwd seeded.
// The dev Vite server proxies /api/v1 to it.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

// The five top-level nav labels the operator should always see in
// the sidebar — group title "Admin" plus four items spanning the
// overview/fleet/platform/admin groups. Kept case-insensitive so a
// stylistic i18n tweak doesn't snap the test.
const NAV_LABELS = ['Home', 'Flows', 'Extensions', 'Admin', 'Settings'] as const

test.describe('chrome', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('login → top-header email + logout → 5 nav sections → logout returns to login slot', async ({
    page,
  }) => {
    // Wide viewport so the md:inline user-email span renders and the
    // sidebar isn't collapsed off-canvas.
    await page.setViewportSize({ width: 1440, height: 900 })

    // ── Login via the unauthenticated slot ────────────────────────
    await page.goto('/')
    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({
      timeout: 10_000,
    })
    await page.locator('#login-email').fill(EMAIL)
    await page.locator('#login-password').fill(PASSWORD)
    await page.getByRole('button', { name: /^Sign in$/ }).click()
    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({
      timeout: 10_000,
    })

    // Wait past the boot intro animation so the chrome is interactive.
    await page.waitForTimeout(3000)

    // Switch to sidebar shell so the AppSidebar is mounted on
    // desktop (in header mode it's mobile-only) — gives us the five
    // nav sections to assert against.
    await page.getByRole('button', { name: /open theme settings/i }).click()
    await expect(page.getByText('Theme Settings')).toBeVisible()
    // The Shell radiogroup carries the "where the primary navigation
    // lives" hint as its aria-label — disambiguates it from the
    // sidebar-variant radiogroup below. The "Sidebar" radio inside
    // it is the i18n value of `layoutToggle.sidebar`.
    await page
      .getByRole('radiogroup', { name: /where the primary navigation lives/i })
      .getByRole('radio', { name: 'Sidebar', exact: true })
      .click()
    await page.keyboard.press('Escape')
    await page.waitForTimeout(500)

    // ── Top-header surfaces email + logout ────────────────────────
    const emailBadge = page.getByTestId('user-email')
    await expect(emailBadge).toHaveText(EMAIL)

    // The logout menu item lives inside the user menu dropdown — open
    // the menu to assert it's present, then close so it doesn't
    // overlap the sidebar assertions below.
    await page.getByRole('button', { name: /account menu/i }).click()
    const logoutItem = page.getByTestId('logout-menu-item')
    await expect(logoutItem).toBeVisible()
    await page.keyboard.press('Escape')

    // ── Left nav has the 5 expected sections ──────────────────────
    const sidebar = page.locator('[data-slot="sidebar"], [data-sidebar="sidebar"]').first()
    await expect(sidebar).toBeVisible()
    for (const label of NAV_LABELS) {
      await expect(
        sidebar.getByText(new RegExp(`^${label}$`, 'i')).first(),
        `sidebar should list ${label}`,
      ).toBeVisible()
    }

    // ── Click logout → unauthenticated slot returns ───────────────
    await page.getByRole('button', { name: /account menu/i }).click()
    await page.getByTestId('logout-menu-item').click()

    await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({
      timeout: 10_000,
    })
    // The app uses AuthProvider.unauthenticatedSlot rather than a
    // dedicated /login route, so the URL stays at /. The login
    // surface re-appearing is the operator-visible "redirect".
    await expect(page).toHaveURL(/\/(?:\?.*)?$/)
  })
})
