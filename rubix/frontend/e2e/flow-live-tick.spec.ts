// Flow live-tick e2e — end-to-end proof of the always-on flow runtime
// shipped by Phase C+D+E. The bundled `com.rubix.tick-counter` flow
// (`trigger.schedule` every 5s → `starter.flow.counter` step=1 → `log`)
// is seeded by `rubix-agent` on boot. This spec exercises three
// promises of the runtime end-to-end:
//
//   1. Live SSE feed — the canvas reflects the counter's `value` slot
//      climbing as ticks fire (count > 0 after one tick window).
//   2. Hot-edit settings — bumping `step` to `10` through the
//      `<SettingsSidebar>` deploys a Settings-classified revision and
//      the engine short-circuits to a slot write; the next tick's
//      value reflects the new step.
//   3. Restart persistence — reloading the page re-subscribes to the
//      same engine; because the counter persists `count` via
//      `NodeStateStore`, the displayed value is preserved (i.e. it
//      does not reset to 0 on refresh).
//
// Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
// bootstrap operator + bundled flow seed. Locally:
//
//     mani run demo
//
// (from `rubix/`). The dev Vite server proxies `/api/v1` to the agent.

import { test, expect, type Page } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'

const TICK_FLOW_ID = 'com.rubix.tick-counter'
const COUNTER_NODE_ID = 'count'
const TICK_PERIOD_MS = 5_000

async function login(page: Page) {
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

/**
 * Pull the current numeric slot value off the counter node. The
 * counter emits an integer on its output slot; `BaseNode` renders it
 * inside `.sf-slot__value`. Returns `null` if the badge is not yet
 * present (no tick has landed in this session).
 */
async function readCounterValue(page: Page): Promise<number | null> {
  const node = page.locator(`[data-node-kind="starter.flow.counter"]`).first()
  const badges = node.locator('.sf-slot__value')
  if ((await badges.count()) === 0) return null
  const text = (await badges.first().textContent())?.trim() ?? ''
  const n = Number(text)
  return Number.isFinite(n) ? n : null
}

async function waitForFirstTick(page: Page): Promise<number> {
  // Allow one full tick window + a margin for boot + SSE handshake.
  return await expect
    .poll(async () => (await readCounterValue(page)) ?? 0, {
      timeout: TICK_PERIOD_MS + 6_000,
      intervals: [500, 750, 1_000],
    })
    .toBeGreaterThan(0)
    .then(async () => (await readCounterValue(page)) as number)
}

test.describe('flow live tick', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('streams ticks, hot-edits step, and preserves count across reloads', async ({
    page,
  }) => {
    await login(page)
    await page.goto(`/flows/${TICK_FLOW_ID}`)

    await expect(
      page.getByRole('heading', { name: new RegExp(TICK_FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    // (1) live SSE — count should climb past 0 within one tick window.
    const firstValue = await waitForFirstTick(page)
    expect(firstValue).toBeGreaterThan(0)

    // (2) hot-edit — click the counter node, bump `step` to 10, save.
    await page
      .locator(`[data-node-kind="starter.flow.counter"]`)
      .first()
      .click()

    await expect(
      page.getByRole('heading', { name: new RegExp(COUNTER_NODE_ID) }),
    ).toBeVisible({ timeout: 5_000 })

    const stepInput = page.locator('#setting-step')
    await expect(stepInput).toBeVisible({ timeout: 5_000 })
    await stepInput.fill('10')

    const baseline = (await readCounterValue(page)) ?? firstValue
    await page.getByRole('button', { name: /^Save$/ }).click()

    // Next emit after save must reflect step=10 — i.e. the value
    // jumps by at least 10 relative to the last pre-save reading.
    await expect
      .poll(async () => (await readCounterValue(page)) ?? baseline, {
        timeout: TICK_PERIOD_MS + 6_000,
        intervals: [500, 750, 1_000],
      })
      .toBeGreaterThanOrEqual(baseline + 10)

    const postEditValue = (await readCounterValue(page)) as number

    // (3) restart persistence — reload the page; the counter is
    // backed by `NodeStateStore` so the next emit should be strictly
    // greater than the pre-reload reading (no reset to 0).
    await page.reload()
    await expect(
      page.getByRole('heading', { name: new RegExp(TICK_FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    await expect
      .poll(async () => (await readCounterValue(page)) ?? 0, {
        timeout: TICK_PERIOD_MS + 6_000,
        intervals: [500, 750, 1_000],
      })
      .toBeGreaterThanOrEqual(postEditValue)
  })
})
