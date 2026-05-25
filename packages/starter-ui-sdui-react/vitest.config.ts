import { defineConfig } from "vitest/config";

// Renderer smokes use `react-dom/server::renderToStaticMarkup` so
// we avoid pulling jsdom into devDeps. Tests that exercise the
// transport seam stub `fetch` directly.
export default defineConfig({
  test: {
    environment: "node",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
