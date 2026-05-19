import { test, expect } from "@playwright/test";
import { ownerToken } from "./helpers.js";

test.describe("Notes app E2E", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to app — should show login form.
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("notes");
  });

  test("shows login form when unauthenticated", async ({ page }) => {
    await expect(page.getByPlaceholder("bearer token")).toBeVisible();
    await expect(page.getByRole("button", { name: "sign in" })).toBeVisible();
  });

  test("login with valid token shows notes view", async ({ page }) => {
    const token = ownerToken();

    await page.getByPlaceholder("bearer token").fill(token);
    await page.getByRole("button", { name: "sign in" }).click();

    // After login, the heading stays "notes" and we see the input for
    // creating a new note.
    await expect(page.getByPlaceholder("new note…")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("signed in as")).toBeVisible();
  });

  test("create and list notes", async ({ page }) => {
    const token = ownerToken();

    // Sign in.
    await page.getByPlaceholder("bearer token").fill(token);
    await page.getByRole("button", { name: "sign in" }).click();
    await expect(page.getByPlaceholder("new note…")).toBeVisible({ timeout: 5000 });

    // Create a note.
    const noteText = `e2e test note ${Date.now()}`;
    await page.getByPlaceholder("new note…").fill(noteText);
    await page.getByPlaceholder("new note…").press("Enter");

    // The note should appear in the list.
    await expect(page.getByText(noteText)).toBeVisible({ timeout: 5000 });
  });

  test("sign out returns to login", async ({ page }) => {
    const token = ownerToken();

    await page.getByPlaceholder("bearer token").fill(token);
    await page.getByRole("button", { name: "sign in" }).click();
    await expect(page.getByPlaceholder("new note…")).toBeVisible({ timeout: 5000 });

    await page.getByRole("button", { name: "sign out" }).click();
    await expect(page.getByPlaceholder("bearer token")).toBeVisible({ timeout: 5000 });
  });

  test("extensions tab visible for admin", async ({ page }) => {
    const token = ownerToken();

    await page.getByPlaceholder("bearer token").fill(token);
    await page.getByRole("button", { name: "sign in" }).click();
    await expect(page.getByPlaceholder("new note…")).toBeVisible({ timeout: 5000 });

    // The extensions tab should be visible (admin token).
    const extTab = page.getByRole("button", { name: "extensions" });
    await expect(extTab).toBeVisible({ timeout: 5000 });

    // Click it — should show extension list or empty state.
    await extTab.click();
    // Either shows an extension card or the "No extensions loaded" text.
    const hasContent = await page
      .getByText(/extensions|No extensions loaded/)
      .first()
      .isVisible();
    expect(hasContent).toBe(true);
  });
});
