import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const here = path.dirname(fileURLToPath(import.meta.url));

// Widget tests resolve their three runtime peers — `@nube/starter-ui-kit-native`,
// `react-native-svg`, and `moti` — to tiny host-element mocks so we can:
//   (a) verify widgets depend ONLY on those surfaces (each mock IS the
//       surface — if a widget reaches for something the mock doesn't
//       export, the test fails at import time);
//   (b) drive widgets under jsdom without bringing real RN.
// The mocks preserve prop pass-through so a11y / animation-config
// assertions are real.
export default defineConfig({
  resolve: {
    alias: {
      "@nube/starter-ui-kit-native": path.resolve(
        here,
        "src/__mocks__/starter-ui-kit-native.tsx",
      ),
      "react-native-svg": path.resolve(
        here,
        "src/__mocks__/react-native-svg.tsx",
      ),
      moti: path.resolve(here, "src/__mocks__/moti.tsx"),
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
