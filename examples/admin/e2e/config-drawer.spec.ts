import { test, expect } from '@playwright/test'

test.describe('ConfigDrawer', () => {
  test.beforeEach(async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`))
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(`console.error: ${msg.text()}`)
    })
    ;(page as any).__errors = errors
  })

  test('gear icon opens Theme Settings drawer with all sections', async ({ page }) => {
    await page.goto('/dashboard')
    await expect(page.getByText('Fleet at a').first()).toBeVisible({ timeout: 10_000 })

    const gear = page.getByRole('button', { name: /open theme settings/i })
    await expect(gear).toBeVisible()
    await gear.click()

    await expect(page.getByText('Theme Settings')).toBeVisible({ timeout: 5_000 })

    for (const section of ['Theme', 'Palette', 'Font', 'Sidebar', 'Layout', 'Direction']) {
      await expect(page.getByText(section, { exact: true }).first()).toBeVisible()
    }
    for (const card of ['System', 'Light', 'Dark', 'Inset', 'Floating']) {
      await expect(page.getByText(card, { exact: true }).first()).toBeVisible()
    }
    for (const font of ['geist', 'inter', 'manrope']) {
      await expect(page.getByRole('radio', { name: new RegExp(`select ${font} font`, 'i') })).toBeVisible()
    }

    const errs = (page as any).__errors as string[]
    expect(errs, `runtime errors:\n${errs.join('\n')}`).toEqual([])
  })

  test('palette swap re-skins via data-palette attribute', async ({ page }) => {
    await page.goto('/dashboard')
    await page.getByRole('button', { name: /open theme settings/i }).click()
    await expect(page.getByText('Theme Settings')).toBeVisible()

    await page.getByRole('radio', { name: /select ocean palette/i }).click()
    await expect(page.locator('html')).toHaveAttribute('data-palette', 'ocean')

    await page.getByRole('radio', { name: /select sunset palette/i }).click()
    await expect(page.locator('html')).toHaveAttribute('data-palette', 'sunset')
  })

  test('font swap updates --font-sans CSS variable', async ({ page }) => {
    await page.goto('/dashboard')
    await page.getByRole('button', { name: /open theme settings/i }).click()
    await expect(page.getByText('Theme Settings')).toBeVisible()

    await page.getByRole('radio', { name: /select inter font/i }).click()
    const fontSans = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--font-sans'),
    )
    expect(fontSans).toContain('Inter')
  })
})
