// Extensions e2e — list renders + SSE-driven row state.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator seeded AND at least one installed extension
// fixture so the table is non-empty. The canonical local bring-up is:
//
//     mani run demo
//
// (from `rubix/`). If the agent reports zero installed extensions
// this spec will fail on the "first row present" assertion — that's a
// fixture problem on the backend, not a frontend regression.

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

test.describe('extensions', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('list renders and start button transitions row state via SSE within 5s', async ({ page }) => {
    await login(page)
    await page.goto('/extensions')

    await expect(page.getByRole('heading', { name: /Installed extensions/i })).toBeVisible({ timeout: 10_000 })

    // Wait for at least one row to land. The row grid uses font-mono
    // for the extension id — pick the first such cell as our anchor.
    const firstRow = page.locator('div.grid', { hasText: /./ }).filter({ has: page.locator('.font-mono') }).first()
    await expect(firstRow).toBeVisible({ timeout: 10_000 })

    // Snapshot the badge text before clicking, so we can assert it
    // actually transitioned (rather than just being equal to its
    // initial value).
    const badge = firstRow.locator('span', { hasText: /^(running|stopped|starting|stopping|errored|installed|enabled|disabled)$/i }).first()
    const before = (await badge.innerText()).trim().toLowerCase()

    const startBtn = firstRow.getByRole('button', { name: /Start/i })
    if (await startBtn.isEnabled()) {
      await startBtn.click()
    } else {
      // Already running — exercise Restart so SSE still ticks.
      await firstRow.getByRole('button', { name: /Restart/i }).click()
    }

    // SSE lifecycle frame should drive a visible badge change within
    // 5 seconds per the stage contract.
    await expect
      .poll(
        async () => (await badge.innerText()).trim().toLowerCase(),
        { timeout: 5_000, intervals: [100, 200, 400] },
      )
      .not.toBe(before)
  })
})
