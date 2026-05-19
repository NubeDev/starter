import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E config for the notes example.
 *
 * Assumes:
 * - The Rust backend is running on :8080 (`cargo run -p starter-notes -- serve`)
 * - Vite dev server is started by `webServer` below on :5173, proxying
 *   API calls (/notes, /auth, /mcp, /extensions, /health) to :8080.
 *
 * Run:
 *   pnpm e2e          # headless
 *   pnpm e2e:ui       # interactive Playwright UI
 */
export default defineConfig({
  testDir: "./e2e",
  globalSetup: "./e2e/global-setup.ts",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: "list",
  timeout: 30_000,

  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: {
    command: "pnpm dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 15_000,
  },
});
