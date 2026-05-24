# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: auth.spec.ts >> auth >> login → dashboard renders → logout → login route shows again
- Location: e2e/auth.spec.ts:26:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: getByRole('heading', { name: /Sign in to Rubix/i })
Expected: visible
Timeout: 10000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 10000ms
  - waiting for getByRole('heading', { name: /Sign in to Rubix/i })

```

```yaml
- banner:
  - link "Nube IoT Console":
    - /url: /
    - img
    - text: Nube IoT Console
  - navigation:
    - link "Home":
      - /url: /
      - img
      - text: Home
    - link "Dashboard":
      - /url: /dashboard
      - img
      - text: Dashboard
    - link "Devices":
      - /url: "#devices"
      - img
      - text: Devices
    - link "Flows":
      - /url: /flows
      - img
      - text: Flows
    - link "Activity":
      - /url: "#activity"
      - img
      - text: Activity
  - img
  - text: No tenant
  - button "Toggle theme":
    - img
  - button "Search… ⌘K":
    - img
    - text: Search… ⌘K
  - 'button "Theme mode: System"':
    - img
  - button "Language":
    - img
  - button "Palette"
  - button "Open theme settings"
  - button "AP":
    - text: AP
    - img
- main:
  - text: Live · 412 devices streaming
  - heading "The operating layer for the physical world." [level=1]
  - paragraph: Stream telemetry, model assets, automate flows, and ship dashboards — all from one extensible Rust runtime. Built for energy, water and HVAC at scale.
  - link "Enter dashboard":
    - /url: /dashboard
    - button "Enter dashboard":
      - text: Enter dashboard
      - img
  - button "Watch 90s tour":
    - img
    - text: Watch 90s tour
  - text: 412 Devices online 3.4k Flows / second 7 Extensions What's inside
  - heading "An IoT platform that gets out of the way." [level=2]
  - link "01 · Flows Visual runtime Visual flows compile to a deterministic Rust runtime. Hot-reload in dev, zero-downtime in prod.":
    - /url: "#"
    - img
    - img
    - text: 01 · Flows
    - heading "Visual runtime" [level=3]
    - paragraph: Visual flows compile to a deterministic Rust runtime. Hot-reload in dev, zero-downtime in prod.
  - link "02 · Extensions Module-Federation Drop-in extensions contribute pages, widgets, and commands without forks. Singletons negotiated automatically.":
    - /url: "#"
    - img
    - img
    - text: 02 · Extensions
    - heading "Module-Federation" [level=3]
    - paragraph: Drop-in extensions contribute pages, widgets, and commands without forks. Singletons negotiated automatically.
  - link "03 · SDUI Server-driven UI Design once, render on web, mobile, and panel. No native rebuild required.":
    - /url: "#"
    - img
    - img
    - text: 03 · SDUI
    - heading "Server-driven UI" [level=3]
    - paragraph: Design once, render on web, mobile, and panel. No native rebuild required.
  - link "04 · Warehouse ClickHouse history Sub-second queries across millions of points and tags. Tags are Bool|Str — L1 to L3 marts on demand.":
    - /url: "#"
    - img
    - img
    - text: 04 · Warehouse
    - heading "ClickHouse history" [level=3]
    - paragraph: Sub-second queries across millions of points and tags. Tags are Bool|Str — L1 to L3 marts on demand.
  - link "05 · AuthZ Per-user gating Dynamic resources, not static routes. Gate any SDUI page per user, per tenant, per role.":
    - /url: "#"
    - img
    - img
    - text: 05 · AuthZ
    - heading "Per-user gating" [level=3]
    - paragraph: Dynamic resources, not static routes. Gate any SDUI page per user, per tenant, per role.
  - link "06 · Git-native Everything is a file Tags, flows, dashboards — all in git. Branch, diff, review, revert.":
    - /url: "#"
    - img
    - img
    - text: 06 · Git-native
    - heading "Everything is a file" [level=3]
    - paragraph: Tags, flows, dashboards — all in git. Branch, diff, review, revert.
  - text: Public preview · Q2 2026
  - heading "Bring your fleet online in an afternoon." [level=2]
  - paragraph: Install the agent, point at your devices, and ship your first dashboard before lunch.
  - link "Open the console":
    - /url: /dashboard
    - button "Open the console":
      - text: Open the console
      - img
  - button "Book a demo":
    - img
    - text: Book a demo
- button "Open Tanstack query devtools":
  - img
```

# Test source

```ts
  1  | // Auth happy-path e2e — login → app shell → logout → login slot re-appears.
  2  | //
  3  | // Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
  4  | // bootstrap operator `op@example.com` / `rubix-dev-passwd` already
  5  | // seeded. The canonical way to bring that up locally is:
  6  | //
  7  | //     mani run demo
  8  | //
  9  | // (from the `rubix/` directory — see `rubix/mani.yaml`'s `demo`
  10 | // task). The dev Vite server proxies `/api/v1` to 127.0.0.1:8088, so
  11 | // the test interacts with the agent through the same origin as the
  12 | // browser.
  13 | 
  14 | import { test, expect } from '@playwright/test'
  15 | 
  16 | const EMAIL = 'op@example.com'
  17 | const PASSWORD = 'rubix-dev-passwd'
  18 | 
  19 | test.describe('auth', () => {
  20 |   test.beforeEach(async ({ context }) => {
  21 |     // Start every spec from a clean cookie jar so the AuthProvider
  22 |     // renders the unauthenticated slot, not a cached session.
  23 |     await context.clearCookies()
  24 |   })
  25 | 
  26 |   test('login → dashboard renders → logout → login route shows again', async ({
  27 |     page,
  28 |     request,
  29 |   }) => {
  30 |     await page.goto('/')
  31 | 
  32 |     // Unauthenticated slot from <AuthProvider>: login form is shown.
> 33 |     await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
     |                                                                            ^ Error: expect(locator).toBeVisible() failed
  34 | 
  35 |     await page.locator('#login-email').fill(EMAIL)
  36 |     await page.locator('#login-password').fill(PASSWORD)
  37 |     await page.getByRole('button', { name: /^Sign in$/ }).click()
  38 | 
  39 |     // After a successful login the AuthProvider swaps the slot for the
  40 |     // routed app. We land on `/` which renders the Landing page — the
  41 |     // "Sign in to Rubix" card must be gone.
  42 |     await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({ timeout: 10_000 })
  43 |     await expect(page).toHaveURL(/\/(?:\?.*)?$/)
  44 | 
  45 |     // Logout via the wire-level endpoint — the UI item in
  46 |     // <NavUser> is not yet wired to `auth.logout()` (see
  47 |     // src/components/layout/nav-user.tsx). The behaviour we care about
  48 |     // here is that the AuthProvider re-renders its unauthenticated
  49 |     // slot after the session cookie clears, regardless of which
  50 |     // surface triggered the logout.
  51 |     const csrf = (await page.context().cookies()).find((c) => /csrf/i.test(c.name))?.value
  52 |     const logout = await request.post('http://127.0.0.1:8088/api/v1/auth/logout', {
  53 |       headers: csrf ? { 'x-csrf-token': csrf } : undefined,
  54 |     })
  55 |     expect(logout.ok()).toBeTruthy()
  56 |     await page.context().clearCookies()
  57 | 
  58 |     await page.goto('/')
  59 |     await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
  60 |   })
  61 | })
  62 | 
```