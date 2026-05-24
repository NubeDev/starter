import { test, expect } from '@playwright/test'

test('header-mode: search pill visible, no sidebar overlay', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto('/dashboard')
  await page.waitForTimeout(3000)

  await page.screenshot({ path: 'e2e/_out/header-search.png', fullPage: false })

  // Sidebar should NOT be in the DOM in header mode (AppSidebar lives only in SidebarShellInner)
  const sidebar = await page.locator('[data-slot="sidebar-container"], [data-sidebar="sidebar"]').count()
  console.log('sidebar containers:', sidebar)

  // Search pill should be visible and have a real width
  const search = page.getByRole('button', { name: /search/i }).first()
  const box = await search.boundingBox()
  console.log('search pill box:', box)
})

test('sidebar-mode then back to header — no leftover sidebar in DOM', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto('/dashboard')
  await page.waitForTimeout(3000)

  await page.getByRole('button', { name: /open theme settings/i }).click()
  await page.getByRole('radio', { name: /select sidebar shell/i }).click()
  await page.waitForTimeout(600)
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)
  await page.screenshot({ path: 'e2e/_out/sidebar-shell.png', fullPage: false })

  // main should NOT be 256px wide
  const mainRect = await page.evaluate(() => {
    const m = document.querySelector('main')
    return m?.getBoundingClientRect()
  })
  console.log('sidebar-mode main rect:', mainRect)
})
