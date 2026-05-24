// Warehouse admin smoke e2e — navigate to /admin/warehouse and
// switch through all 4 tabs of <WarehouseAdmin>, asserting that the
// page heading renders and each tab becomes selected with a panel
// marker visible. This is a shell-mount smoke that catches i18n
// mis-wiring, broken provider hierarchy, missing exports from the
// rubix-side warehouse admin barrel, or a panel that crashes on
// mount. We do NOT exercise the underlying clickhouse/insights
// tools — the panels may show empty states (no rows in a fresh
// dev backend) and that is acceptable for the smoke.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator `op@example.com` / `rubix-dev-passwd` seeded.
// Locally:
//
//     mani run demo
//
// (from `rubix/`). The dev Vite server proxies `/api/v1` to the
// agent.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

// Per-tab: the TabsTrigger label (role="tab") and a marker that is
// rendered inside the corresponding TabsContent panel. The marker
// is either an always-visible control (e.g. the "New mart" button)
// or an empty-state title that the panel renders when its backing
// list is empty. We accept either by using regex/text matches that
// only appear in the active panel.
const TABS: { trigger: RegExp; marker: RegExp }[] = [
  { trigger: /^Rules$/,     marker: /No projection rules|Use the rubix\.clickhouse\.rule\.write/i },
  { trigger: /^Marts$/,     marker: /New mart/i },
  { trigger: /^Retention$/, marker: /No tables|TTL \(days\)/i },
  { trigger: /^Insights$/,  marker: /New insights rule/i },
]

async function login(page: import('@playwright/test').Page) {
  await page.goto('/')
  await expect(
    page.getByRole('heading', { name: /Sign in to Rubix/i }),
  ).toBeVisible({ timeout: 10_000 })
  await page.locator('#login-email').fill(EMAIL)
  await page.locator('#login-password').fill(PASSWORD)
  await page.getByRole('button', { name: /^Sign in$/ }).click()
  await expect(
    page.getByRole('heading', { name: /Sign in to Rubix/i }),
  ).toBeHidden({ timeout: 10_000 })
}

test.describe('admin/warehouse', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('navigates to /admin/warehouse and every tab renders its panel marker', async ({ page }) => {
    await login(page)
    await page.goto('/admin/warehouse')

    // The rubix wrapper renders the page heading.
    await expect(
      page.getByRole('heading', { name: /^Warehouse$/, level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    for (const { trigger, marker } of TABS) {
      const tab = page.getByRole('tab', { name: trigger })
      await tab.click()
      await expect(tab).toHaveAttribute('aria-selected', 'true', { timeout: 5_000 })
      await expect(page.getByText(marker).first()).toBeVisible({ timeout: 10_000 })
    }
  })
})
