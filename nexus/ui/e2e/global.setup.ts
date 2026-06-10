import { test as setup, expect } from "@playwright/test";

// Log in once (cookie + CSRF, the app's real auth) and persist the browser
// storage state so every test starts authenticated. The dev server proxies
// /auth to nexus-api, so we hit it through the UI origin and the session
// cookie lands on the right domain.
const EMAIL = process.env.NEXUS_ADMIN_EMAIL ?? "admin@nexus.local";
const PASSWORD = process.env.NEXUS_ADMIN_PASSWORD ?? "change-me-admin";

setup("authenticate", async ({ page }) => {
  // POST the login through the page's request context so Set-Cookie is stored
  // in this browser context.
  const res = await page.request.post("/auth/login", {
    headers: { "content-type": "application/json" },
    data: { email: EMAIL, password: PASSWORD },
  });
  expect(res.ok(), `login failed: ${res.status()} ${await res.text()}`).toBeTruthy();

  // Sanity: the session cookie now authenticates /api/v1/me.
  const me = await page.request.get("/api/v1/me");
  expect(me.ok(), `/me not authed: ${me.status()}`).toBeTruthy();

  await page.context().storageState({ path: "e2e/.auth/state.json" });
});
