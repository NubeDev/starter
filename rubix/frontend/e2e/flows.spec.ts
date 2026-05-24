// Flows smoke e2e — navigate to `/flows`, assert the deployed-flow
// list renders at least 6 rows (the bundled flows seeded by rubix-agent
// via PR #32, plus any subsequent rows), then click into
// `com.rubix.scheduled-system-check` and assert the FlowCanvas mounts
// with at least one node whose kind label reads `ai-agent`.
//
// The list shell, registry wiring, xyflow stylesheet, and rubix-side
// `ai-agent` override (which still composes `BaseNode`, so the
// `.sf-node__kind` span renders the literal kind id) are all
// exercised end-to-end.
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator + bundled flow seed. Locally:
//
//     mani run demo
//
// (from `rubix/`). The dev Vite server proxies `/api/v1` to the agent.

import { test, expect } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

const TARGET_FLOW_ID = 'com.rubix.scheduled-system-check'

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

test.describe('flows', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('lists bundled flows and renders an ai-agent node in the canvas', async ({ page }) => {
    await login(page)
    await page.goto('/flows')

    await expect(
      page.getByRole('heading', { name: /Deployed flows/i, level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    // The table renders one <tbody><tr> per flow. The 6 bundled flows
    // from PR #32 must all be visible; the assertion uses `>= 6` so
    // additional rows added later don't break the smoke.
    const rows = page.locator('table tbody tr')
    await expect.poll(() => rows.count(), { timeout: 15_000 }).toBeGreaterThanOrEqual(6)

    // Click into the target flow. The id is rendered inside the
    // `Flow id` column as a link cell.
    await page.getByRole('link', { name: new RegExp(TARGET_FLOW_ID) }).click()

    await expect(
      page.getByRole('heading', { name: new RegExp(TARGET_FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    // `BaseNode` from `@nube/starter-ui-flow` renders the node kind
    // verbatim inside `<span class="sf-node__kind">`. The placeholder
    // graph synthesised by `useFlowDefinition` always carries one
    // `ai-agent` node, and the rubix override still composes BaseNode
    // — so this selector works whether the body endpoint is live or
    // still stubbed.
    await expect(page.locator('.sf-node__kind', { hasText: 'ai-agent' }).first())
      .toBeVisible({ timeout: 15_000 })
  })
})
