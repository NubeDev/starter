import { test, expect } from '@playwright/test'

test('header-mode dashboard is centered, not squashed left', async ({ page }) => {
  await page.setViewportSize({ width: 2560, height: 1080 })
  await page.goto('/dashboard')
  await page.waitForTimeout(3000) // boot intro

  await page.screenshot({ path: 'e2e/_out/layout-header-mode.png', fullPage: false })

  // The H1 "Fleet at a glance" should be horizontally centered-ish on a 2560 viewport
  const h1 = page.getByRole('heading', { name: /Fleet at a/i })
  const box = await h1.boundingBox()
  expect(box).not.toBeNull()
  if (!box) return

  console.log(`H1 x=${box.x} width=${box.width}, viewport=2560`)
  // The h1 sits inside max-w-7xl (1280px) centered → left edge should be ~640px in a 2560 viewport
  expect(box.x).toBeGreaterThan(400)
})
