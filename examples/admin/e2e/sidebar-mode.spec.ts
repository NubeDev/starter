import { test, expect } from '@playwright/test'

test('switching to sidebar mode keeps dashboard content visible', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}\n${e.stack}`))
  page.on('console', (m) => { if (m.type() === 'error') errors.push(`console.error: ${m.text()}`) })

  await page.setViewportSize({ width: 2560, height: 1080 })
  await page.goto('/dashboard')
  await page.waitForTimeout(3000)

  // open drawer and switch shell to sidebar
  await page.getByRole('button', { name: /open theme settings/i }).click()
  await expect(page.getByText('Theme Settings')).toBeVisible()
  await page.getByRole('radio', { name: /select sidebar shell/i }).click()
  await page.waitForTimeout(800)

  // close drawer (Escape)
  await page.keyboard.press('Escape')
  await page.waitForTimeout(400)

  await page.screenshot({ path: 'e2e/_out/sidebar-mode.png', fullPage: false })

  // check the dashboard content is visible
  const h1 = page.getByRole('heading', { name: /Fleet at a/i })
  const visible = await h1.isVisible().catch(() => false)
  console.log('H1 visible:', visible)
  if (visible) {
    const box = await h1.boundingBox()
    console.log('H1 box:', box)
  }

  // dump the main element
  const mainHtml = await page.evaluate(() => {
    const m = document.querySelector('main')
    return { exists: !!m, innerText: m?.innerText?.slice(0, 200), rect: m?.getBoundingClientRect() }
  })
  console.log('MAIN:', JSON.stringify(mainHtml, null, 2))

  if (errors.length) console.log('ERRORS:\n', errors.join('\n'))
})
