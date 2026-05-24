# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: authz-admin.spec.ts >> admin/access >> navigates to /admin/access and every tab renders its heading
- Location: e2e/authz-admin.spec.ts:55:3

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
  1  | // Authz admin smoke e2e — navigate to /admin/access and switch
  2  | // through all 8 tabs of <AuthzAdmin>, asserting each tab's heading
  3  | // renders without error. This is a shell-mount smoke; we do NOT
  4  | // assert business logic (creating tenants, evaluating rules, etc.)
  5  | // — only that clicking each tab swaps the panel and renders its
  6  | // title. That's enough to catch i18n mis-wiring, missing exports
  7  | // from @nube/starter-ui-authz, broken provider hierarchy, or a
  8  | // panel that crashes on mount.
  9  | //
  10 | // Prerequisite: a running `rubix-agent` on 127.0.0.1:8088 with the
  11 | // bootstrap operator `op@example.com` / `rubix-dev-passwd` seeded.
  12 | // Locally:
  13 | //
  14 | //     mani run demo
  15 | //
  16 | // (from `rubix/`). The dev Vite server proxies `/api/v1` to the
  17 | // agent. The operator must have a role that lets the authz read
  18 | // endpoints respond (otherwise the panels still mount but with
  19 | // error rows — which is fine for this smoke; we don't gate on
  20 | // success of the underlying fetches).
  21 | 
  22 | import { test, expect } from '@playwright/test'
  23 | 
  24 | const EMAIL = 'op@example.com'
  25 | const PASSWORD = 'rubix-dev-passwd'
  26 | 
  27 | // Tab trigger label (visible text on the TabsTrigger button) and the
  28 | // h2 heading the corresponding panel renders. Trigger and heading
  29 | // share the same text for most tabs — we disambiguate by role.
  30 | const TABS: { trigger: RegExp; heading: RegExp }[] = [
  31 |   { trigger: /^Tenants$/,     heading: /^Tenants$/ },
  32 |   { trigger: /^Members$/,     heading: /^Members$/ },
  33 |   { trigger: /^Teams$/,       heading: /^Teams$/ },
  34 |   { trigger: /^Rules$/,       heading: /^Rules$/ },
  35 |   { trigger: /^Assignments$/, heading: /^Assignments$/ },
  36 |   { trigger: /^Resources$/,   heading: /^Resources$/ },
  37 |   { trigger: /^Check$/,       heading: /Dry-run check/i },
  38 |   { trigger: /^Decisions$/,   heading: /^Decisions$/ },
  39 | ]
  40 | 
  41 | async function login(page: import('@playwright/test').Page) {
  42 |   await page.goto('/')
> 43 |   await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeVisible({ timeout: 10_000 })
     |                                                                          ^ Error: expect(locator).toBeVisible() failed
  44 |   await page.locator('#login-email').fill(EMAIL)
  45 |   await page.locator('#login-password').fill(PASSWORD)
  46 |   await page.getByRole('button', { name: /^Sign in$/ }).click()
  47 |   await expect(page.getByRole('heading', { name: /Sign in to Rubix/i })).toBeHidden({ timeout: 10_000 })
  48 | }
  49 | 
  50 | test.describe('admin/access', () => {
  51 |   test.beforeEach(async ({ context }) => {
  52 |     await context.clearCookies()
  53 |   })
  54 | 
  55 |   test('navigates to /admin/access and every tab renders its heading', async ({ page }) => {
  56 |     await login(page)
  57 |     await page.goto('/admin/access')
  58 | 
  59 |     // The page-level heading from the rubix wrapper.
  60 |     await expect(
  61 |       page.getByRole('heading', { name: /Access Control/i, level: 1 }),
  62 |     ).toBeVisible({ timeout: 10_000 })
  63 | 
  64 |     for (const { trigger, heading } of TABS) {
  65 |       // TabsTrigger is rendered as role="tab" (Radix). Clicking it
  66 |       // mounts the corresponding TabsContent panel.
  67 |       await page.getByRole('tab', { name: trigger }).click()
  68 |       await expect(
  69 |         page.getByRole('heading', { name: heading, level: 2 }),
  70 |       ).toBeVisible({ timeout: 10_000 })
  71 |     }
  72 |   })
  73 | })
  74 | 
```