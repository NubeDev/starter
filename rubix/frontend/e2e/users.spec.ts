// Users admin e2e — create → list reflects → undo via UI reverts.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator `op@example.com` / `rubix-dev-passwd` seeded.
// Locally:
//
//     mani run demo
//
// (from `rubix/`). The bootstrap operator must have admin role so
// that the user-management endpoints don't 403.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

async function login(page: import('@playwright/test').Page) {
  await page.goto('/')
  await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
  await page.locator('#login-email').fill(EMAIL)
  await page.locator('#login-password').fill(PASSWORD)
  await page.getByRole('button', { name: /^Sign in$/ }).click()
  await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({ timeout: 10_000 })
}

test.describe('admin/users', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('create flow lands, list reflects, undo via UI button reverts', async ({ page }) => {
    await login(page)
    await page.goto('/admin/users')

    await expect(page.getByRole('heading', { name: /^Users$/ })).toBeVisible({ timeout: 10_000 })

    // Unique email per run so re-running against the same DB doesn't
    // collide with a prior create.
    const ts = Date.now()
    const email = `e2e-create-${ts}@example.com`

    await page.locator('#user-email').fill(email)
    // `#user-role` is pre-filled with 'operator' — keep the default.

    await page.getByRole('button', { name: /^Create$/ }).click()

    // List should reflect the new user once `useUserCreate` finishes
    // and invalidates `['rubix','users']`.
    const newRow = page.locator('div.grid', { hasText: email })
    await expect(newRow).toBeVisible({ timeout: 10_000 })

    // Undo via the page-header button — should call
    // `useUndoLast({})` and revert the create. The row goes away
    // entirely (hard delete via undo) OR flips to a "Disabled"
    // status — accept either to keep the spec robust against the
    // exact semantics on the backend.
    await page.getByRole('button', { name: /Undo last/i }).click()

    await expect
      .poll(
        async () => {
          if ((await newRow.count()) === 0) return 'gone'
          const txt = (await newRow.innerText()).toLowerCase()
          return txt.includes('disabled') ? 'disabled' : 'present'
        },
        { timeout: 10_000, intervals: [200, 400, 800] },
      )
      .not.toBe('present')
  })
})
