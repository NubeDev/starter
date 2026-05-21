import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Renderer smokes use `react-dom/server::renderToStaticMarkup`
    // — no DOM needed, so we avoid pulling jsdom into the package's
    // dev deps. The Custom-fallback smoke (R7) lives in
    // `src/components/Custom.test.tsx`.
    environment: "node",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
