import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The HTTP adapter uses standard WHATWG fetch / Response /
    // ReadableStream / TextDecoder — all native in modern Node, no
    // DOM needed.
    environment: "node",
    globals: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
