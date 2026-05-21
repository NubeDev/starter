/**
 * Page Builder E2E — drives the fixture-only Page Builder slice end
 * to end. Covers the eight PAGE-BUILDER.md acceptance checks that
 * can be observed from the outside:
 *
 *  1. empty state on first visit
 *  2. `sales` prompt streams a tree (no "Unknown component" leak)
 *  3. buffered-patch badge briefly visible during the turn
 *  4. save → /pages/:id renders the same tree
 *  5. sidebar Pages section updates live
 *  6. Edit ⇄ View round-trip preserves the tree
 *  7. console reports no errors during the run
 *  8. /skills shows the two seeded bundles
 */
import { test, expect } from "@playwright/test";

const PROMPT_PLACEHOLDER =
  "Describe the UI… try: sales · dashboard · onboard · report";

test.beforeEach(async ({ page }) => {
  // Start each test from a clean slate so `usePages()` reports empty.
  // The init script runs on EVERY navigation in the same context, so
  // we gate the localStorage wipe on a sessionStorage flag — that way
  // the first page load clears, but subsequent navigations within the
  // same test (Save → /pages/:id → /edit) keep the seeded data.
  await page.addInitScript(() => {
    try {
      if (!sessionStorage.getItem("__e2eCleared")) {
        sessionStorage.setItem("__e2eCleared", "1");
        window.localStorage.removeItem("flow-agent:pages");
      }
    } catch {
      /* private mode etc. */
    }
    // Race-free check for the buffered-patch badge: a MutationObserver
    // installed at document creation time catches every text update,
    // so a badge that flips on/off inside ~160 ms still leaves a
    // trace on `window.__seenBufferedBadge` once the turn finishes.
    (window as unknown as { __seenBufferedBadge?: boolean }).__seenBufferedBadge = false;
    const check = () => {
      if (/\d+\s*buffered/i.test(document.body.innerText)) {
        (window as unknown as { __seenBufferedBadge?: boolean }).__seenBufferedBadge = true;
      }
    };
    const obs = new MutationObserver(check);
    const start = () =>
      obs.observe(document.documentElement, {
        subtree: true,
        childList: true,
        characterData: true,
      });
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", start, { once: true });
    } else {
      start();
    }
  });
});

test("renders the empty state on first visit to /pages", async ({ page }) => {
  await page.goto("/pages");
  await expect(
    page.getByRole("heading", { name: "Pages", level: 2 }),
  ).toBeVisible();
  await expect(page.getByText("No pages yet", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", { name: /new page/i }).first(),
  ).toBeVisible();
});

test("sales prompt streams a renderable tree, save round-trips, sidebar updates", async ({
  page,
}) => {
  // Surface any console error so the test fails loudly on the
  // `Unknown component: container` class of regressions.
  const consoleErrors: string[] = [];
  page.on("pageerror", (err) => consoleErrors.push(String(err)));
  page.on("console", (msg) => {
    if (msg.type() === "error") consoleErrors.push(msg.text());
  });

  await page.goto("/pages/new?fixture=1");
  await expect(page.getByText(/page builder/i)).toBeVisible();

  // Send the `sales` prompt via the chat composer.
  const composer = page.getByPlaceholder(PROMPT_PLACEHOLDER);
  await composer.fill("sales");
  await page.getByRole("button", { name: "Send" }).click();

  // The fixture lands `status: done` at ~880ms; allow a generous
  // wall-clock budget for CI jitter.
  const canvas = page.locator('[data-slot="ai-builder-canvas"]').first();
  await expect(canvas).toBeVisible();
  await expect(canvas.getByText("Sales · Q2", { exact: true })).toBeVisible({
    timeout: 5_000,
  });
  await expect(canvas.getByText("MRR")).toBeVisible();
  await expect(canvas.getByText("$42k")).toBeVisible();
  await expect(canvas.getByRole("table")).toBeVisible();
  await expect(canvas.getByText("Qualified")).toBeVisible();

  // No "Unknown component" placeholder leaked from the renderer.
  await expect(page.getByText(/unknown component/i)).toHaveCount(0);

  // Name + Save (the disabled state lifts once the tree exists).
  await page.getByPlaceholder("Page name").fill("Sales · Q2 E2E");
  const saveBtn = page.getByRole("button", { name: /save page/i });
  await expect(saveBtn).toBeEnabled();
  await saveBtn.click();

  // Lands on /pages/:id with the same tree.
  await expect(page).toHaveURL(/\/pages\/[A-Za-z0-9_-]+$/);
  await expect(page.getByText("Sales · Q2", { exact: true })).toBeVisible();
  await expect(page.getByText("$42k")).toBeVisible();
  await expect(page.getByRole("table")).toBeVisible();

  // Sidebar reflects the new page (Shell subscribes via usePages()).
  const sidebar = page.locator('[data-slot="sidebar"]').first();
  await expect(sidebar.getByText("Sales · Q2 E2E")).toBeVisible();

  expect(consoleErrors, consoleErrors.join("\n")).toEqual([]);
});

test("buffered-patch badge appears during the sales turn", async ({ page }) => {
  await page.goto("/pages/new?fixture=1");
  await page.getByPlaceholder(PROMPT_PLACEHOLDER).fill("sales");
  await page.getByRole("button", { name: "Send" }).click();

  // Wait for the turn to finish so we know all mutations have flushed,
  // then ask the MutationObserver installed in beforeEach whether the
  // badge ever rendered. This is race-free regardless of fixture timing.
  const canvas = page.locator('[data-slot="ai-builder-canvas"]').first();
  await expect(canvas.getByText("Sales · Q2", { exact: true })).toBeVisible({
    timeout: 8_000,
  });
  await expect.poll(async () =>
    page.evaluate(
      () =>
        (window as unknown as { __seenBufferedBadge?: boolean })
          .__seenBufferedBadge === true,
    ),
  ).toBe(true);
});

test("edit round-trip preserves the saved tree", async ({ page }) => {
  // Seed one page directly so we can jump straight to /edit.
  await page.goto("/pages/new?fixture=1");
  await page.getByPlaceholder(PROMPT_PLACEHOLDER).fill("report");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(
    page.getByText("Daily report", { exact: true }).first(),
  ).toBeVisible({ timeout: 8_000 });
  await page.getByPlaceholder("Page name").fill("Daily report E2E");
  await page.getByRole("button", { name: /save page/i }).click();
  await expect(page).toHaveURL(/\/pages\/[A-Za-z0-9_-]+$/);

  // The view should render the table.
  await expect(page.getByRole("table")).toBeVisible();
  const viewRows = await page.getByRole("table").locator("tbody tr").count();

  // Switch to edit mode via direct navigation (the view exposes an
  // Edit affordance but we want the round-trip assertion to depend
  // only on the seeded tree, not the chrome around it).
  const viewUrl = page.url();
  await page.goto(`${viewUrl}/edit`);
  await expect(page).toHaveURL(/\/edit$/);
  const canvas = page.locator('[data-slot="ai-builder-canvas"]').first();
  await expect(canvas.getByRole("table")).toBeVisible();
  const editRows = await canvas.getByRole("table").locator("tbody tr").count();
  expect(editRows).toBe(viewRows);
  await expect(page.getByText(/unknown component/i)).toHaveCount(0);
});

test("skills page lists the two seeded reference bundles", async ({ page }) => {
  await page.goto("/skills");
  await expect(
    page.getByRole("heading", { name: /skills/i }).first(),
  ).toBeVisible();
  await expect(
    page.getByText("starter.ai-builder.dashboards").first(),
  ).toBeVisible();
  await expect(
    page.getByText("starter.ai-builder.themes").first(),
  ).toBeVisible();
});
