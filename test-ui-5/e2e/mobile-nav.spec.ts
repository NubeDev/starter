import { test, expect, devices } from '@playwright/test'

const MOBILE_VIEWPORT = devices['Pixel 7'].viewport ?? { width: 412, height: 915 }

test.use({ viewport: MOBILE_VIEWPORT })

/**
 * Mobile nav reuses the shadcn `AppSidebar` in both layout modes:
 *   - header mode: <AppSidebar /> is mounted only when useIsMobile() is true,
 *     and the hamburger in <TopHeader/> is a <SidebarTrigger> with md:hidden.
 *   - sidebar mode: <AppSidebar /> is the desktop sidebar; on mobile it
 *     collapses to the same off-canvas Sheet.
 * Both surfaces render `[data-mobile="true"]` when open.
 */

async function setLayoutMode(context: import('@playwright/test').BrowserContext, mode: 'header' | 'sidebar') {
  await context.addCookies([
    { name: 'layout_mode', value: mode, url: 'http://localhost:5177' },
  ])
}

test.describe('Mobile navigation — shared shadcn sidebar', () => {
  test('header mode: hamburger opens the shadcn sidebar sheet', async ({ page, context }) => {
    await setLayoutMode(context, 'header')
    await page.goto('/')

    const trigger = page.getByTestId('mobile-nav-trigger')
    await expect(trigger).toBeVisible({ timeout: 10_000 })

    // Inline desktop nav stays hidden at mobile width.
    await expect(page.locator('header nav.md\\:flex').first()).toBeHidden()

    await trigger.click()
    const sheet = page.locator('[data-mobile="true"]')
    await expect(sheet).toBeVisible()

    // All three NAV_GROUPS render as SidebarGroupLabel inside the sheet.
    for (const group of ['Overview', 'Fleet', 'Platform']) {
      await expect(sheet.getByText(group, { exact: true })).toBeVisible()
    }
  })

  test('header mode: tapping a route link navigates and closes the sheet', async ({ page, context }) => {
    await setLayoutMode(context, 'header')
    await page.goto('/')
    await page.getByTestId('mobile-nav-trigger').click()

    const sheet = page.locator('[data-mobile="true"]')
    await expect(sheet).toBeVisible()
    await sheet.getByRole('link', { name: /^Dashboard/ }).click()

    await expect(page).toHaveURL(/\/dashboard$/)
    await expect(sheet).toBeHidden()
  })

  test('header mode: pressing Escape dismisses the sheet without navigating', async ({ page, context }) => {
    await setLayoutMode(context, 'header')
    await page.goto('/dashboard')
    await page.getByTestId('mobile-nav-trigger').click()

    const sheet = page.locator('[data-mobile="true"]')
    await expect(sheet).toBeVisible()

    await page.keyboard.press('Escape')
    await expect(sheet).toBeHidden()
    await expect(page).toHaveURL(/\/dashboard$/)
  })

  test('sidebar mode: existing sidebar trigger still opens the same sheet on mobile', async ({ page, context }) => {
    await setLayoutMode(context, 'sidebar')
    await page.goto('/dashboard')

    const trigger = page.getByRole('button', { name: /toggle sidebar/i }).first()
    await expect(trigger).toBeVisible({ timeout: 10_000 })
    await trigger.click()

    const sheet = page.locator('[data-mobile="true"]')
    await expect(sheet).toBeVisible()
    await expect(sheet.getByText('Overview', { exact: true })).toBeVisible()
  })

  test('sidebar mode: tapping a route link in the sheet navigates and closes', async ({ page, context }) => {
    await setLayoutMode(context, 'sidebar')
    await page.goto('/')

    await page.getByRole('button', { name: /toggle sidebar/i }).first().click()
    const sheet = page.locator('[data-mobile="true"]')
    await expect(sheet).toBeVisible()

    await sheet.getByRole('link', { name: /^Dashboard/ }).click()
    await expect(page).toHaveURL(/\/dashboard$/)
    await expect(sheet).toBeHidden()
  })
})
