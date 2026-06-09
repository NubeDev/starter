import "@testing-library/jest-dom/vitest";

// jsdom ships neither observer; components that rely on them (ECharts'
// auto-resize, Framer Motion's in-view) reference them at mount. These
// are no-op test doubles so pure components mount without a real layout
// engine — test infrastructure, not app behaviour.
class NoopObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() {
    return [];
  }
}

const g = globalThis as unknown as Record<string, unknown>;
g.ResizeObserver ??= NoopObserver;
g.IntersectionObserver ??= NoopObserver;

// jsdom has no canvas backend; ECharts calls `getContext("2d")` at init.
// A stub returning a Proxy of no-op methods lets canvas panels mount in
// tests (we assert the option-building logic separately, not pixels).
const ctx2d = new Proxy(
  {},
  {
    get: (_t, prop) =>
      prop === "canvas"
        ? document.createElement("canvas")
        : prop === "measureText"
          ? () => ({ width: 0 })
          : prop === "getImageData"
            ? () => ({ data: new Uint8ClampedArray(4) })
            : prop === "createLinearGradient" || prop === "createRadialGradient"
              ? () => ({ addColorStop() {} })
              : () => undefined,
  },
);
HTMLCanvasElement.prototype.getContext =
  (() => ctx2d) as unknown as HTMLCanvasElement["getContext"];
