import { test, expect } from "@playwright/test";
import { ownerToken } from "./helpers.js";

// /settings/theme smoke. The Theme tab now mounts the full
// `<ThemeEditorPage>` from `@nube/starter-ui-kit`: a horizontal
// preset gallery (data-testid="theme-gallery"), a token/branding
// tab pair, a live preview pane, and a "Save" button that talks to
// `PUT /api/v1/ui/theme`. The flow exercised here:
//
// 1. sign in
// 2. switch to the Theme tab
// 3. click the "Modern Minimal" preset (by accessible name)
// 4. press Save
// 5. verify the `PUT` round-trips with an `oklch(...)` primary
// 6. verify the document-level `--primary` CSS var actually changed.

test.describe("Theme settings E2E", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("notes");
  });

  test("admin can apply a preset and the server persists it", async ({ page }) => {
    const token = ownerToken();
    await page.getByPlaceholder("bearer token").fill(token);
    await page.getByRole("button", { name: /sign in/i }).click();

    const themeTab = page.getByRole("button", { name: /^theme$/i });
    await expect(themeTab).toBeVisible({ timeout: 5000 });
    await themeTab.click();

    // Gallery is the load-bearing element.
    await expect(page.getByTestId("theme-gallery")).toBeVisible({ timeout: 5000 });

    const beforePrimary = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue("--primary").trim(),
    );

    // Picking a preset hydrates the store but doesn't auto-save
    // (the editor surfaces an explicit Save button so admin can
    // preview before committing).
    await page.getByRole("button", { name: /modern minimal/i }).first().click();

    const [putResponse] = await Promise.all([
      page.waitForResponse(
        (res) =>
          res.url().endsWith("/api/v1/ui/theme") &&
          res.request().method() === "PUT" &&
          res.status() === 200,
        { timeout: 5000 },
      ),
      page.getByRole("button", { name: /^save$/i }).click(),
    ]);

    const body = await putResponse.json();
    expect(body.theme_styles.light.primary).toMatch(/^oklch\(/);

    const afterPrimary = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue("--primary").trim(),
    );
    expect(afterPrimary).not.toBe(beforePrimary);
    expect(afterPrimary).toMatch(/^oklch\(/);
  });
});
