// Flow-editor round-trip e2e — verifies the canvas reflects backend
// state without a manual reload after a deploy.
//
// Three assertions, all on the bundled `com.rubix.tick-counter` flow:
//   1. All three nodes (`trigger.schedule`, `counter`, `log`) render
//      `BaseNode` chrome, including `.sf-slot` handles. This is the
//      proof that the `starter.flow.log` + `starter.flow.trigger.schedule`
//      UI specs are wired in the registry. Without them, xyflow falls
//      back to its default node which has no slot handles.
//   2. Deleting a node from the canvas (Backspace on the selected
//      node) persists to the backend: the canvas updates in place,
//      `body_yaml` from `useFlowsList()` re-fetches, and the node
//      stays gone after a hard reload.
//   3. The pre-fix bug — "must refresh the page to see backend
//      state" — does NOT regress: after the delete, the on-screen
//      node count immediately matches the backend YAML without any
//      manual `page.reload()`.
//
// Pre-req: same as `flows.spec.ts` (live rubix-agent on 8088 +
// bundled flow seed via `mani run demo`).

import { test, expect, type Page } from '@playwright/test'

const EMAIL = 'op@example.com'
const PASSWORD = 'rubix-dev-passwd'
const FLOW_ID = 'com.rubix.tick-counter'

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
 * Canonical bundled body for `com.rubix.tick-counter`. Hardcoded so
 * each test starts from the same 3-node / 2-edge graph regardless
 * of what prior tests mutated. Mirrors
 * `rubix/crates/rubix-flows/flows/tick-counter.yaml`.
 */
const TICK_COUNTER_BODY = `id: com.rubix.tick-counter
description: |
  Always-on demo flow. A schedule trigger fires every 5 seconds; the
  counter increments and emits the new value; the log node records it.
trigger: schedule
cron_expr: "*/5 * * * * *"

nodes:
  - id: tick
    kind: starter.flow.trigger.schedule
    config:
      cron_expr: "*/5 * * * * *"
  - id: count
    kind: starter.flow.counter
    config:
      step: 1
      initial: 0
      reset_on_redeploy: false
  - id: emit
    kind: starter.flow.log
    config:
      level: info

links:
  - { from: "tick.fire", to: "count.in" }
  - { from: "count.out", to: "emit.value" }
`

async function resetTickCounter(page: Page): Promise<void> {
  // Login via the API so we have cookies for the deploy POST without
  // bouncing through the SPA login.
  await page.context().request.post('/api/v1/auth/login', {
    data: { email: EMAIL, password: PASSWORD },
  })
  const r = await page.context().request.post(
    '/api/v1/tools/rubix.flow_ops.deploy',
    { data: { flow_id: FLOW_ID, body_yaml: TICK_COUNTER_BODY } },
  )
  expect(r.ok()).toBe(true)
}

test.describe('flow editor — round-trip', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies()
  })

  test('renders handles for all built-in kinds and deletes in place', async ({ page }) => {
    await login(page)
    await resetTickCounter(page)

    await page.goto(`/flows/${FLOW_ID}`)

    // Header proves the route mounted.
    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    // The marketing boot intro mounts a fixed-position `z-[100]`
    // overlay for ~2.4s on first paint that swallows pointer events.
    await page
      .locator('div.fixed.inset-0.z-\\[100\\]')
      .waitFor({ state: 'detached', timeout: 5_000 })

    // (1) Three BaseNode frames + handles for every kind.
    const nodes = page.locator('.sf-node')
    await expect.poll(() => nodes.count(), { timeout: 15_000 }).toBe(3)

    for (const kind of [
      'starter.flow.trigger.schedule',
      'starter.flow.counter',
      'starter.flow.log',
    ]) {
      const node = page.locator(`.sf-node[data-node-kind="${kind}"]`)
      await expect(node).toBeVisible()
      // SlotHandle renders `.sf-slot` per slot. trigger.schedule has
      // 1 output, log has 1 input, counter has 1 in + 1 out — all > 0.
      const slots = node.locator('.sf-slot')
      await expect.poll(() => slots.count()).toBeGreaterThan(0)
      // xyflow `<Handle>` renders `.react-flow__handle`; confirm at
      // least one is present so the connect-by-drag surface is real.
      const handles = node.locator('.react-flow__handle')
      await expect.poll(() => handles.count()).toBeGreaterThan(0)
    }

    // (2) Select the log node and delete it. xyflow listens for
    // selection on `.react-flow__node` (the wrapper it owns); the
    // `.sf-node` we render lives inside, so clicking the wrapper
    // is the most reliable way to drive selection from a test.
    const logWrapper = page
      .locator('.react-flow__node')
      .filter({ has: page.locator('[data-node-kind="starter.flow.log"]') })
      .first()
    await logWrapper.waitFor({ state: 'visible' })
    await logWrapper.scrollIntoViewIfNeeded()
    // `force: true` skips hit-testing — useful when xyflow's
    // fit-view animation or a transient overlay would otherwise
    // intercept the click. We've already scoped to the exact node
    // wrapper, so there's no risk of clicking the wrong element.
    await logWrapper.click({ force: true })
    // Confirm xyflow registered the selection before pressing the
    // delete key — RF marks the wrapper with `.selected`.
    await expect(logWrapper).toHaveClass(/selected/, { timeout: 5_000 })
    // xyflow's default `deleteKeyCode` accepts Backspace; some
    // configurations override to Delete. Fire both — whichever it
    // listens to triggers `onNodesChange` with a `remove` event.
    await page.keyboard.press('Backspace')
    await page.keyboard.press('Delete')

    // (3) In-place update: the on-screen count drops to 2 with NO
    // manual reload. This is the regression guard for the
    // useFlowGraph `initial` sync.
    await expect.poll(() => nodes.count(), { timeout: 10_000 }).toBe(2)
    await expect(page.locator('.sf-node[data-node-kind="starter.flow.log"]')).toHaveCount(0)

    // Hard reload to prove the delete really persisted to YAML.
    await page.reload()
    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })
    await expect.poll(() => page.locator('.sf-node').count(), { timeout: 15_000 }).toBe(2)
    await expect(page.locator('.sf-node[data-node-kind="starter.flow.log"]')).toHaveCount(0)
  })

  test('connects two handles by drag and persists the new edge', async ({ page }) => {
    await login(page)
    await resetTickCounter(page)
    await page.goto(`/flows/${FLOW_ID}`)

    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })

    // The marketing boot intro mounts a fixed-position `z-[100]`
    // overlay for ~2.4s on first paint, swallowing any pointer
    // event we'd dispatch onto the canvas. Wait for it to fully
    // unmount before driving the drag.
    await page
      .locator('div.fixed.inset-0.z-\\[100\\]')
      .waitFor({ state: 'detached', timeout: 5_000 })

    // Wait for the canvas to layout. xyflow sets `data-handlepos` on
    // each `<Handle>`; we use it to scope to source/target sides.
    await expect.poll(() => page.locator('.sf-node').count(), { timeout: 15_000 }).toBe(3)

    // Snapshot the current YAML link count via the API. We add one
    // new edge (`tick.fire → emit.value`, both `any`-compatible),
    // so the expected post-deploy count is base + 1.
    const before = await page.context().request.post(
      '/api/v1/tools/rubix.flow_ops.list',
      { data: {} },
    )
    const beforeJson = (await before.json()) as {
      flows: Array<{ flow_id: string; body_yaml: string }>
    }
    const beforeYaml = beforeJson.flows.find((f) => f.flow_id === FLOW_ID)!.body_yaml
    const beforeLinkCount = (beforeYaml.match(/^\s+-\s+\{?\s*from:/gm) ?? []).length
    expect(beforeLinkCount).toBeGreaterThanOrEqual(2)

    // Find the source handle on the `tick` (schedule) node and the
    // target handle on the `emit` (log) node. xyflow renders one
    // `.react-flow__handle.source` per output and `.target` per input.
    const tickNode = page
      .locator('.react-flow__node')
      .filter({ has: page.locator('[data-node-kind="starter.flow.trigger.schedule"]') })
    const emitNode = page
      .locator('.react-flow__node')
      .filter({ has: page.locator('[data-node-kind="starter.flow.log"]') })
    const sourceHandle = tickNode.locator('.react-flow__handle.source').first()
    const targetHandle = emitNode.locator('.react-flow__handle.target').first()

    await sourceHandle.waitFor({ state: 'visible' })
    await targetHandle.waitFor({ state: 'visible' })

    // xyflow's connection state machine listens on pointer events.
    // Dispatch raw pointer events (not just mouse) so the source
    // handle's `onPointerDown` fires its connect logic. Playwright's
    // `page.mouse` only emits mouse events on Chromium, which xyflow
    // ignores for handles in v12.
    const srcBox = (await sourceHandle.boundingBox())!
    const dstBox = (await targetHandle.boundingBox())!
    const srcX = srcBox.x + srcBox.width / 2
    const srcY = srcBox.y + srcBox.height / 2
    const dstX = dstBox.x + dstBox.width / 2
    const dstY = dstBox.y + dstBox.height / 2

    await page.evaluate(
      ({ srcX, srcY, dstX, dstY }) => {
        const fire = (target: Element, type: string, x: number, y: number) => {
          const init: PointerEventInit = {
            bubbles: true,
            cancelable: true,
            composed: true,
            pointerType: 'mouse',
            isPrimary: true,
            clientX: x,
            clientY: y,
            screenX: x,
            screenY: y,
            button: 0,
            buttons: type === 'pointerup' ? 0 : 1,
          }
          target.dispatchEvent(new PointerEvent(type, init))
          // xyflow v12 also listens on mouse* in some browsers; fire
          // both flavours for safety.
          const mouseType = type.replace('pointer', 'mouse')
          target.dispatchEvent(
            new MouseEvent(mouseType, {
              ...init,
              view: window,
            }),
          )
        }
        const start = document.elementFromPoint(srcX, srcY)
        const end = document.elementFromPoint(dstX, dstY)
        if (!start || !end) throw new Error('handle elements not under cursor')
        fire(start, 'pointerdown', srcX, srcY)
        const steps = 12
        for (let i = 1; i <= steps; i += 1) {
          const x = srcX + ((dstX - srcX) * i) / steps
          const y = srcY + ((dstY - srcY) * i) / steps
          const mid = document.elementFromPoint(x, y) ?? document.body
          fire(mid, 'pointermove', x, y)
        }
        fire(end, 'pointerup', dstX, dstY)
      },
      { srcX, srcY, dstX, dstY },
    )

    // Wait for the deploy round-trip to bump the edge count both
    // on-screen (one more `.react-flow__edge`) and in the backend
    // YAML.
    await expect.poll(
      async () => {
        const r = await page.context().request.post(
          '/api/v1/tools/rubix.flow_ops.list',
          { data: {} },
        )
        const j = (await r.json()) as {
          flows: Array<{ flow_id: string; body_yaml: string }>
        }
        const y = j.flows.find((f) => f.flow_id === FLOW_ID)!.body_yaml
        return (y.match(/^\s+-\s+\{?\s*from:/gm) ?? []).length
      },
      { timeout: 15_000, intervals: [200, 500, 1000] },
    ).toBe(beforeLinkCount + 1)
  })

  test('persists node positions across reloads', async ({ page }) => {
    await login(page)
    await resetTickCounter(page)
    await page.goto(`/flows/${FLOW_ID}`)

    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })
    await page
      .locator('div.fixed.inset-0.z-\\[100\\]')
      .waitFor({ state: 'detached', timeout: 5_000 })
    await expect.poll(() => page.locator('.sf-node').count(), { timeout: 15_000 }).toBe(3)

    // Snapshot the initial canvas-space position of the `count`
    // node so we can compute a known-good drag delta.
    const countWrapper = page
      .locator('.react-flow__node')
      .filter({ has: page.locator('[data-node-kind="starter.flow.counter"]') })
      .first()
    const before = (await countWrapper.boundingBox())!
    const dx = 140
    const dy = -90

    // Drag by the node's header chrome (anything inside `.sf-node`
    // is xyflow-draggable; the handles carry `nodrag`).
    const header = countWrapper.locator('.sf-node__header').first()
    const headerBox = (await header.boundingBox())!
    const startX = headerBox.x + headerBox.width / 2
    const startY = headerBox.y + headerBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX + dx / 2, startY + dy / 2, { steps: 8 })
    await page.mouse.move(startX + dx, startY + dy, { steps: 8 })
    await page.mouse.up()

    // Verify the canvas moved by approximately (dx, dy). xyflow's
    // viewport may apply non-unit zoom from fitView; we only need
    // to prove the node moved in the expected direction, not the
    // exact distance. The persisted-position assertion below is
    // the real proof.
    const after = (await countWrapper.boundingBox())!
    expect(after.x).not.toBe(before.x)
    expect(after.y).not.toBe(before.y)
    expect(Math.sign(after.x - before.x)).toBe(Math.sign(dx))
    expect(Math.sign(after.y - before.y)).toBe(Math.sign(dy))

    // Wait for the position to land in the backend YAML and
    // capture the persisted (x, y) for the post-reload comparison.
    type Pos = { x: number; y: number }
    const readCountPos = async (): Promise<Pos | null> => {
      const r = await page.context().request.post(
        '/api/v1/tools/rubix.flow_ops.list',
        { data: {} },
      )
      const j = (await r.json()) as {
        flows: Array<{ flow_id: string; body_yaml: string }>
      }
      const y = j.flows.find((f) => f.flow_id === FLOW_ID)!.body_yaml
      const m = y.match(
        /id:\s*count[\s\S]*?position:\s*\{?\s*x:\s*(-?\d+)\s*,\s*y:\s*(-?\d+)/m,
      )
      return m ? { x: Number(m[1]), y: Number(m[2]) } : null
    }
    await expect.poll(readCountPos, { timeout: 15_000, intervals: [200, 500, 1000] })
      .not.toBeNull()
    const persisted = (await readCountPos())!

    // Reload — the YAML position must still be the same number.
    // (Screen-space bounding boxes drift between mounts because
    // xyflow's fitView re-runs with a different viewport, so we
    // assert on the persisted source-of-truth, not the pixels.)
    await page.reload()
    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })
    await page
      .locator('div.fixed.inset-0.z-\\[100\\]')
      .waitFor({ state: 'detached', timeout: 5_000 })
    await expect.poll(() => page.locator('.sf-node').count(), { timeout: 15_000 }).toBe(3)

    const afterReload = (await readCountPos())!
    expect(afterReload).toEqual(persisted)
  })

  test('deletes a selected edge with Backspace and persists the removal', async ({
    page,
  }) => {
    await login(page)
    await resetTickCounter(page)
    await page.goto(`/flows/${FLOW_ID}`)

    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })
    await page
      .locator('div.fixed.inset-0.z-\\[100\\]')
      .waitFor({ state: 'detached', timeout: 5_000 })

    // Helper: read the YAML link count straight from the backend.
    const readLinkCount = async (): Promise<number> => {
      const r = await page.context().request.post(
        '/api/v1/tools/rubix.flow_ops.list',
        { data: {} },
      )
      const j = (await r.json()) as {
        flows: Array<{ flow_id: string; body_yaml: string }>
      }
      const y = j.flows.find((f) => f.flow_id === FLOW_ID)!.body_yaml
      return (y.match(/^\s+-\s+\{?\s*from:/gm) ?? []).length
    }

    // The bundled flow ships with two links; that's the baseline.
    const before = await readLinkCount()
    expect(before).toBe(2)

    await expect.poll(() => page.locator('.react-flow__edge').count(), {
      timeout: 15_000,
    }).toBe(2)

    // Select the first edge — clicking the visible SVG path works
    // for xyflow's selection in v12 (the wider hit-area path under
    // the visible one is also part of `.react-flow__edge`).
    const edge = page.locator('.react-flow__edge').first()
    await edge.click({ force: true })
    await expect(edge).toHaveClass(/selected/, { timeout: 5_000 })

    await page.keyboard.press('Backspace')

    // In-place update: one fewer edge on the canvas, and the
    // backend YAML lost the same link.
    await expect.poll(() => page.locator('.react-flow__edge').count(), {
      timeout: 10_000,
    }).toBe(1)
    await expect.poll(readLinkCount, {
      timeout: 15_000,
      intervals: [200, 500, 1000],
    }).toBe(before - 1)

    // Hard reload — the removal must survive the round-trip.
    await page.reload()
    await expect(
      page.getByRole('heading', { name: new RegExp(FLOW_ID), level: 1 }),
    ).toBeVisible({ timeout: 10_000 })
    await expect.poll(() => page.locator('.react-flow__edge').count(), {
      timeout: 15_000,
    }).toBe(1)
  })
})
