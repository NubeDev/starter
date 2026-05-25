// Dashboards smoke e2e — exercises the Phase D.2 plumbing end-to-end.
//
//   1. login
//   2. navigate /dashboards/disk-overview
//   3. assert the KPI tile renders a numeric value
//   4. assert the chart SVG mounts
//   5. click a time-range toggle (when present)
//   6. assert `dashboard.page_set` lands (no network 4xx/5xx)
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bundled `disk-overview` page seeded by `boot::dashboards_seed`.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

async function login(page: import('@playwright/test').Page) {
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
}

test.describe('dashboards', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('renders disk-overview kpi + chart + page_set round-trip', async ({ page }) => {
    await login(page)

    // Watch every page_set XHR — the click on the time-range toggle
    // is expected to issue one; assertion at the end confirms 2xx.
    const pageSetResponses: number[] = []
    page.on('response', (resp) => {
      if (resp.url().includes('/api/v1/tools/rubix.dashboard.page_set')) {
        pageSetResponses.push(resp.status())
      }
    })

    await page.goto('/dashboards/disk-overview')

    // KPI tile — the `<RenderKpi>` widget exposes a numeric value
    // through `data-sdui-kind="kpi"` with the numeric text inside.
    const kpi = page.locator('[data-sdui-kind="kpi"]').first()
    await expect(kpi).toBeVisible({ timeout: 15_000 })
    await expect(kpi).toContainText(/\d/)

    // Chart — every chart renderer mounts an `<svg>` somewhere inside.
    const chart = page.locator('[data-sdui-kind="chart"] svg').first()
    await expect(chart).toBeVisible({ timeout: 15_000 })

    // Time-range toggle, when the page authors one. Best-effort: the
    // bundled disk-overview keeps a 1D / 1W / 1M / 1Y row when the
    // renderer provides one. The click drives `page_set` through the
    // engine slot-write chokepoint.
    const range = page.getByRole('button', { name: /1W/ }).first()
    if (await range.count()) {
      await range.click()
      await expect
        .poll(() => pageSetResponses.length, { timeout: 5_000 })
        .toBeGreaterThan(0)
      for (const status of pageSetResponses) {
        expect(status, 'page_set landed with 2xx').toBeLessThan(400)
      }
    }
  })
})
