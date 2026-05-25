import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const here = path.dirname(fileURLToPath(import.meta.url));

// Renderer tests resolve `@nube/starter-ui-kit-native` to a tiny
// host-element mock so we can:
//   (a) verify renderers depend only on the kit surface (the mock is
//       the surface — if a renderer calls something the mock doesn't
//       export the test breaks at import time);
//   (b) drive renderers under jsdom without bringing real RN.
// The mock preserves prop pass-through so a11y assertions are real.
export default defineConfig({
  resolve: {
    alias: {
      "@nube/starter-ui-kit-native": path.resolve(
        here,
        "src/__mocks__/starter-ui-kit-native.tsx",
      ),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
