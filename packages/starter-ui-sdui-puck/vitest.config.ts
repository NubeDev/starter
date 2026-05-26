import { defineConfig } from "vitest/config";

// Mirrors @nube/starter-ui-sdui-react's vitest setup — node env, no
// jsdom. The config generator is pure; the PuckBuilder stub doesn't
// touch the DOM in PR1. resolveJsonModule lets us import the
// committed IR schema artifact directly.
export default defineConfig({
  test: {
    environment: "node",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
