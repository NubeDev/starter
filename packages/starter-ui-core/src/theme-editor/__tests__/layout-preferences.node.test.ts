// Sanity test for Phase-1 / Stage-1 of the rubix-mobile workspace
// refactor: the `matchMedia` guards in `layout-preferences.ts`
// (L81, L92, L104) must no-op cleanly when neither `window` nor
// `window.matchMedia` is present. RN's Hermes hits the same guard
// branch — if the guard ever regresses to `?.()` instead of the
// explicit `typeof` check, native consumers crash on bundle load.
//
// We can't switch this file's environment to `node` via Vitest's
// per-file `// @vitest-environment node` pragma because the shared
// `vitest.config.ts` sets `jsdom` globally; instead we stub the
// jsdom-provided globals to simulate the RN runtime shape and
// restore them in `afterEach`. The functional outcome is identical:
// the guards must take the early-return path.

import { afterEach, describe, expect, it } from "vitest";

import {
  resolveMode,
  subscribePrefersDark,
  subscribePrefersReducedMotion,
} from "../layout-preferences.js";

type GlobalRef = Record<string, unknown>;

const g = globalThis as unknown as GlobalRef;
const originalWindow = g.window as (Window & typeof globalThis) | undefined;

afterEach(() => {
  // Restore whatever jsdom installed so the other tests keep passing.
  if (originalWindow === undefined) {
    delete g.window;
  } else {
    g.window = originalWindow;
  }
});

describe("layout-preferences matchMedia guards (RN/Hermes shape)", () => {
  it("resolveMode('system') falls back to light when window is undefined", () => {
    // Simulate Hermes: no `window` global at all.
    delete g.window;
    expect(resolveMode("system")).toBe("light");
  });

  it("resolveMode('system') falls back to light when matchMedia is missing", () => {
    // Simulate a stripped runtime where `window` exists but lacks
    // matchMedia (some embedded webviews, older RN web).
    const cloned = { ...originalWindow } as Record<string, unknown>;
    delete cloned.matchMedia;
    g.window = cloned;
    expect(resolveMode("system")).toBe("light");
  });

  it("subscribePrefersDark returns an unsubscribe no-op when window is undefined", () => {
    delete g.window;
    const unsub = subscribePrefersDark(() => {
      throw new Error("listener must never fire on RN/Hermes");
    });
    expect(typeof unsub).toBe("function");
    // Calling the no-op must not throw.
    expect(() => unsub()).not.toThrow();
  });

  it("subscribePrefersReducedMotion returns an unsubscribe no-op when matchMedia is missing", () => {
    const cloned = { ...originalWindow } as Record<string, unknown>;
    delete cloned.matchMedia;
    g.window = cloned;
    const unsub = subscribePrefersReducedMotion(() => {
      throw new Error("listener must never fire on RN/Hermes");
    });
    expect(typeof unsub).toBe("function");
    expect(() => unsub()).not.toThrow();
  });
});
