import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // jsdom so component tests (React Flow / Testing Library) can run later;
    // the pure-logic tests under lib/ don't touch the DOM and run fine here too.
    environment: "jsdom",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
