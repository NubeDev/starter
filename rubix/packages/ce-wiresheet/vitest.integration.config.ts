import { defineConfig } from "vitest/config";

// Integration tests (*.itest.ts) drive the REAL Control Engine over REST + the
// binary WebSocket — they require a live CE with the NubeIO math extension.
// Run separately from the hermetic unit suite:  pnpm test:integration
// Point at a CE with CE_BASE (default http://127.0.0.1:7878). When the CE is
// unreachable the suites skip themselves rather than fail.
export default defineConfig({
  test: {
    environment: "node", // global fetch + WebSocket (Node 22); no DOM needed
    globals: false,
    include: ["src/**/*.itest.ts"],
    testTimeout: 20000,
    hookTimeout: 20000,
    // One CE, shared mutable state → don't run integration files in parallel.
    fileParallelism: false,
  },
});
