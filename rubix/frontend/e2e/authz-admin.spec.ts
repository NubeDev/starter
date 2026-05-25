// Authz admin smoke e2e — navigate to /admin/access and switch
// through all 8 tabs of <AuthzAdmin>, asserting each tab's heading
// renders without error. This is a shell-mount smoke; we do NOT
// assert business logic (creating tenants, evaluating rules, etc.)
// — only that clicking each tab swaps the panel and renders its
// title. That's enough to catch i18n mis-wiring, missing exports
// from @nube/starter-ui-authz, broken provider hierarchy, or a
// panel that crashes on mount.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator `op@example.com` / `rubix-dev-passwd` seeded.
// Locally:
//
//     mani run demo
//
// (from `rubix/`). The dev Vite server proxies `/api/v1` to the
// agent. The operator must have a role that lets the authz read
// endpoints respond (otherwise the panels still mount but with
// error rows — which is fine for this smoke; we don't gate on
// success of the underlying fetches).

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

// Tab trigger label (visible text on the TabsTrigger button) and the
// h2 heading the corresponding panel renders. Trigger and heading
// share the same text for most tabs — we disambiguate by role.
const TABS: { trigger: RegExp; heading: RegExp }[] = [
  { trigger: /^Tenants$/,     heading: /^Tenants$/ },
  { trigger: /^Members$/,     heading: /^Members$/ },
  { trigger: /^Teams$/,       heading: /^Teams$/ },
  { trigger: /^Rules$/,       heading: /^Rules$/ },
  { trigger: /^Assignments$/, heading: /^Assignments$/ },
  { trigger: /^Resources$/,   heading: /^Resources$/ },
  { trigger: /^Check$/,       heading: /Dry-run check/i },
  { trigger: /^Decisions$/,   heading: /^Decisions$/ },
]

async function login(page: import('@playwright/test').Page) {
  await page.goto('/')
  await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
  await page.locator('#login-email').fill(EMAIL)
  await page.locator('#login-password').fill(PASSWORD)
  await page.getByRole('button', { name: /^Sign in$/ }).click()
  await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({ timeout: 10_000 })
}

test.describe('admin/access', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('navigates to /admin/access and every tab renders its heading', async ({ page }) => {
    await login(page)
    await page.goto('/admin/access')

    // The page-level heading from the rubix wrapper.
    await expect(
      page.getByRole('heading', { name: /Access Control/i, level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    for (const { trigger, heading } of TABS) {
      // TabsTrigger is rendered as role="tab" (Radix). Clicking it
      // mounts the corresponding TabsContent panel.
      await page.getByRole('tab', { name: trigger }).click()
      await expect(
        page.getByRole('heading', { name: heading, level: 2 }),
      ).toBeVisible({ timeout: 10_000 })
    }
  })

  // Regression: the Tabs primitive (@nube/starter-ui-kit) previously
  // used Tailwind bare-key data variants (`data-horizontal:flex-col`,
  // `data-active:...`) that compile to `[data-horizontal]` selectors.
  // Radix emits `data-orientation="horizontal"` and `data-state="active"`,
  // not `data-horizontal`/`data-active`, so the variants silently
  // failed and TabsContent rendered as a right-side column beside
  // TabsList instead of stacked below it.
  // Asserts the active panel's bounding box sits below the tab list.
  test('TabsContent renders below TabsList, not beside it', async ({ page }) => {
    await login(page)
    await page.goto('/admin/access')

    const list = page.getByRole('tablist').first()
    await expect(list).toBeVisible({ timeout: 10_000 })
    const panel = page.locator('[role="tabpanel"][data-state="active"]').first()
    await expect(panel).toBeVisible({ timeout: 10_000 })

    const lb = await list.boundingBox()
    const cb = await panel.boundingBox()
    expect(lb).not.toBeNull()
    expect(cb).not.toBeNull()
    expect(cb!.y).toBeGreaterThan(lb!.y + lb!.height - 1)
  })
})
