import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const here = path.dirname(fileURLToPath(import.meta.url));

// Under vitest we run in jsdom and substitute the native peer deps
// with a thin host-element mock. This keeps unit tests fast and
// dependency-free while still rendering the actual primitive code
// path (`Pressable`, `TextInput`, `MotiView`, etc.). The mock
// preserves prop pass-through so accessibility assertions are real.
export default defineConfig({
  resolve: {
    alias: {
      "react-native": path.resolve(here, "src/__mocks__/react-native.tsx"),
      "react-native-svg": path.resolve(here, "src/__mocks__/react-native-svg.tsx"),
      "react-native-reanimated": path.resolve(
        here,
        "src/__mocks__/react-native-reanimated.ts",
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
