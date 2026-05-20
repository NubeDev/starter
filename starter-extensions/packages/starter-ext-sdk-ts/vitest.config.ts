import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // jsdom because the Stage 3 hooks (`useHostPrefs`,
    // `useHostTranslate`, `useHostFormatters`) and the
    // `MockHostProvider` test helper mount panels via
    // `@testing-library/react`, which needs a DOM. The other SDK
    // tests (e.g. `register.test.ts`) are tolerant of jsdom too.
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
