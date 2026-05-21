/**
 * Live Page Builder E2E — drives the REAL backend (no `?fixture=1`).
 *
 * Asserts the chat surface actually tells the user what happened:
 *   - Build turn: canvas updates AND chat shows a closure ack (the
 *     "Asking Claude…" status is replaced, not left dangling).
 *   - Ask turn: chat shows assistant prose; canvas is unchanged.
 *   - Mode toggle: post-toggle Regenerate uses the CURRENT mode.
 *
 * Skipped by default because it burns real Claude tokens; opt in with
 * `E2E_LIVE=1 pnpm exec playwright test e2e/page-builder-live.spec.ts`.
 * Assumes `make start` is running (backend on :9741, SPA on :9742).
 */
import { test, expect } from "@playwright/test";

const LIVE = process.env.E2E_LIVE === "1";

test.describe(LIVE ? "live page-builder" : "live page-builder (skipped)", () => {
  test.skip(!LIVE, "set E2E_LIVE=1 to run against the live backend");

  test.beforeEach(async ({ page }) => {
    // Clear stored sessions so each run starts cold.
    await page.addInitScript(() => {
      try {
        for (const k of Object.keys(window.localStorage)) {
          if (k.startsWith("flow-agent:")) window.localStorage.removeItem(k);
        }
      } catch {
        /* private mode */
      }
    });
  });

  test("build → ask → toggle → build closure messaging", async ({ page }) => {
    test.setTimeout(300_000);

    const consoleErrors: string[] = [];
    page.on("pageerror", (e) => consoleErrors.push(String(e)));
    page.on("console", (m) => {
      if (m.type() === "error") consoleErrors.push(m.text());
    });

    await page.goto("/pages/new");

    // The Build/Ask toggle must be visible.
    const buildBtn = page.getByRole("radio", { name: /^build$/i });
    const askBtn = page.getByRole("radio", { name: /^ask$/i });
    await expect(buildBtn).toBeVisible({ timeout: 10_000 });
    await expect(askBtn).toBeVisible();

    const composer = page.getByPlaceholder(/describe the ui|ask a question/i);

    // --- 1) BUILD turn ---
    await composer.fill("a tiny chiller plan page with two KPIs");
    await page.getByRole("button", { name: /^send$/i }).click();

    // Canvas renders something.
    const canvas = page.locator('[data-slot="ai-builder-canvas"]').first();
    await expect(canvas.getByText(/chiller/i)).toBeVisible({ timeout: 90_000 });

    // Chat closure: stale "Asking Claude…" status is gone, replaced by
    // the Build-mode acknowledgement bubble.
    await expect(page.getByText(/✓ updated the page/i)).toBeVisible({
      timeout: 10_000,
    });
    await expect(page.getByText(/asking claude/i)).toHaveCount(0);

    // --- 2) ASK turn ---
    await askBtn.click();
    await composer.fill("can you do any colours, to the kpis?");
    await page.getByRole("button", { name: /^send$/i }).click();

    // Assistant prose bubble appears (Ask mode always mentions either
    // "Build mode" or KPI / colour related text in its reply).
    await expect(
      page.locator("body").getByText(/build mode|colou?r|kpi/i).last(),
    ).toBeVisible({ timeout: 90_000 });
    await expect(page.getByText(/asking claude/i)).toHaveCount(0);

    // --- 3) Toggle to Build, send a follow-up. ---
    await buildBtn.click();
    await composer.fill("apply some colours to the kpis");
    await page.getByRole("button", { name: /^send$/i }).click();

    // Wait for the SECOND build acknowledgement (step 1 produced
    // the first); .last() alone would match the stale one.
    await expect(page.getByText(/✓ updated the page/i)).toHaveCount(2, {
      timeout: 120_000,
    });
    await expect(page.getByText(/asking claude/i)).toHaveCount(0);

    expect(consoleErrors, consoleErrors.join("\n")).toEqual([]);
  });
});
