import { defineConfig, devices } from "@playwright/test";

// E2E config for the chart-settings tests. Drives the running dev UI (:4790),
// which proxies /auth + /api to nexus-api (:4780). Auth is cookie+CSRF, so a
// global setup logs in once and saves the storage state every test reuses.
//
// Prereqs (not started by this config — the stack is long-lived in dev):
//   - nexus-api on :4780  (make dev-be FEATURES=zenoh)
//   - vite UI on :4790    (make dev-ui)
//   - a seeded admin + the `energy` dashboard with panels
const BASE = process.env.NEXUS_UI_URL ?? "http://127.0.0.1:4790";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: BASE,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    // setup logs in and WRITES the storage state — it must not try to load it.
    { name: "setup", testMatch: /global\.setup\.ts/ },
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], storageState: "e2e/.auth/state.json" },
      dependencies: ["setup"],
    },
  ],
});
