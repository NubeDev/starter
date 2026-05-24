import { test } from '@playwright/test'

test('debug: screenshot dashboard, click gear, screenshot drawer', async ({ page }) => {
  const logs: string[] = []
  page.on('console', (m) => logs.push(`[${m.type()}] ${m.text()}`))
  page.on('pageerror', (e) => logs.push(`[pageerror] ${e.message}\n${e.stack}`))

  await page.goto('/dashboard', { waitUntil: 'networkidle' })
  await page.screenshot({ path: 'e2e/_out/01-dashboard.png', fullPage: true })

  // dump everything near the top-right where the gear should be
  const dock = await page.evaluate(() => {
    const el = document.querySelector('header')
    return el?.outerHTML?.slice(0, 4000) ?? '<no header>'
  })
  console.log('HEADER HTML:', dock)

  const gear = page.getByRole('button', { name: /open theme settings/i })
  const count = await gear.count()
  console.log('GEAR COUNT:', count)

  if (count > 0) {
    await gear.first().scrollIntoViewIfNeeded()
    await page.screenshot({ path: 'e2e/_out/02-pre-click.png', fullPage: true })
    await gear.first().click({ trial: false })
    await page.waitForTimeout(800)
    await page.screenshot({ path: 'e2e/_out/03-post-click.png', fullPage: true })

    const drawerText = await page.evaluate(
      () => document.body.innerText.includes('Theme Settings'),
    )
    console.log('DRAWER VISIBLE:', drawerText)
  }

  console.log('--- BROWSER LOGS ---')
  for (const l of logs) console.log(l)
})
