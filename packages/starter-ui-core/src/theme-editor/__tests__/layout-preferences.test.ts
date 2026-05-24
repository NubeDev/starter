// Stage 2 additions: layout preferences + apply helper + Tailwind v4
// generator. Each test covers one of the four new exports — store,
// resolveMode + media-query helpers, applyThemePreferences,
// generateTailwindThemeCss.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  DENSITY_SCALE,
  FONT_SIZE_SCALE,
  defaultLayoutPreferences,
  resolveMode,
} from "../layout-preferences.js";
import { createLayoutPreferencesStore } from "../layout-preferences-store.js";
import { applyThemePreferences, clearThemePreferences } from "../utils/apply-preferences.js";
import { generateTailwindThemeCss } from "../utils/tailwind-css.js";

describe("resolveMode", () => {
  it("passes light/dark through unchanged", () => {
    expect(resolveMode("light")).toBe("light");
    expect(resolveMode("dark")).toBe("dark");
  });

  it("collapses system to light when prefers-color-scheme misses", () => {
    const spy = vi
      .spyOn(window, "matchMedia")
      .mockImplementation(() => ({ matches: false }) as unknown as MediaQueryList);
    expect(resolveMode("system")).toBe("light");
    spy.mockRestore();
  });

  it("collapses system to dark when prefers-color-scheme matches", () => {
    const spy = vi
      .spyOn(window, "matchMedia")
      .mockImplementation(() => ({ matches: true }) as unknown as MediaQueryList);
    expect(resolveMode("system")).toBe("dark");
    spy.mockRestore();
  });
});

describe("applyThemePreferences", () => {
  let el: HTMLElement;
  beforeEach(() => {
    el = document.createElement("div");
  });

  it("writes data attributes + CSS vars from preferences", () => {
    const result = applyThemePreferences(el, {
      mode: "dark",
      density: "compact",
      fontSize: "lg",
      motion: "reduced",
      palette: "nube",
    });
    expect(result.resolvedMode).toBe("dark");
    expect(el.getAttribute("data-mode")).toBe("dark");
    expect(el.classList.contains("dark")).toBe(true);
    expect(el.getAttribute("data-palette")).toBe("nube");
    expect(el.getAttribute("data-motion")).toBe("reduced");
    expect(el.getAttribute("data-density")).toBe("compact");
    expect(el.getAttribute("data-font-size")).toBe("lg");
    expect(el.style.getPropertyValue("--density-scale")).toBe(String(DENSITY_SCALE.compact));
    expect(el.style.getPropertyValue("--font-size-scale")).toBe(String(FONT_SIZE_SCALE.lg));
  });

  it("clears the dark class and palette when light + null palette", () => {
    el.classList.add("dark");
    el.setAttribute("data-palette", "ocean");
    applyThemePreferences(el, {
      ...defaultLayoutPreferences,
      mode: "light",
      palette: null,
    });
    expect(el.classList.contains("dark")).toBe(false);
    expect(el.hasAttribute("data-palette")).toBe(false);
  });

  it("clearThemePreferences removes every attribute and var", () => {
    applyThemePreferences(el, { ...defaultLayoutPreferences, mode: "dark", palette: "ocean" });
    clearThemePreferences(el);
    expect(el.hasAttribute("data-mode")).toBe(false);
    expect(el.hasAttribute("data-palette")).toBe(false);
    expect(el.hasAttribute("data-motion")).toBe(false);
    expect(el.hasAttribute("data-density")).toBe(false);
    expect(el.hasAttribute("data-font-size")).toBe(false);
    expect(el.classList.contains("dark")).toBe(false);
    expect(el.style.getPropertyValue("--density-scale")).toBe("");
    expect(el.style.getPropertyValue("--font-size-scale")).toBe("");
  });
});

describe("generateTailwindThemeCss", () => {
  it("emits an @theme block for light tokens and a nested .dark block", () => {
    const css = generateTailwindThemeCss({
      light: { primary: "oklch(0.7 0.1 200)", radius: "0.5rem" },
      dark: { primary: "oklch(0.3 0.1 200)" },
    });
    expect(css).toContain("@theme inline {");
    expect(css).toContain("--color-primary: oklch(0.7 0.1 200);");
    expect(css).toContain("--radius: 0.5rem;");
    expect(css).toContain(".dark {");
    expect(css).toContain("--color-primary: oklch(0.3 0.1 200);");
  });

  it("omits the .dark block when dark map is empty", () => {
    const css = generateTailwindThemeCss({
      light: { primary: "oklch(0.7 0.1 200)" },
      dark: {},
    });
    expect(css).not.toContain(".dark {");
  });

  it("uses --color- prefix for colour tokens and bare name for shape/font", () => {
    const css = generateTailwindThemeCss({
      light: {
        background: "oklch(1 0 0)",
        "font-sans": '"Inter", sans-serif',
        radius: "0.75rem",
        "shadow-blur": "10px",
      },
      dark: {},
    });
    expect(css).toContain("--color-background:");
    expect(css).toContain("--font-sans:");
    expect(css).toContain("--radius:");
    expect(css).toContain("--shadow-blur:");
    expect(css).not.toContain("--color-font-sans");
    expect(css).not.toContain("--color-radius");
  });
});

describe("LayoutPreferences store", () => {
  // Each test gets an isolated store with in-memory storage so we
  // don't pollute localStorage between vitest workers.
  let useStore: ReturnType<typeof createLayoutPreferencesStore>;

  beforeEach(() => {
    useStore = createLayoutPreferencesStore({ storage: null });
  });

  afterEach(() => {
    useStore.persist?.clearStorage?.();
  });

  it("seeds from defaultLayoutPreferences", () => {
    const s = useStore.getState();
    expect(s.mode).toBe(defaultLayoutPreferences.mode);
    expect(s.density).toBe(defaultLayoutPreferences.density);
    expect(s.fontSize).toBe(defaultLayoutPreferences.fontSize);
    expect(s.motion).toBe(defaultLayoutPreferences.motion);
    expect(s.palette).toBe(defaultLayoutPreferences.palette);
  });

  it("setters update the matching field", () => {
    useStore.getState().setMode("dark");
    useStore.getState().setDensity("compact");
    useStore.getState().setFontSize("lg");
    useStore.getState().setMotion("reduced");
    useStore.getState().setPalette("ocean");
    const s = useStore.getState();
    expect(s.mode).toBe("dark");
    expect(s.density).toBe("compact");
    expect(s.fontSize).toBe("lg");
    expect(s.motion).toBe("reduced");
    expect(s.palette).toBe("ocean");
  });

  it("hydrate replaces the whole preference object", () => {
    useStore.getState().hydrate({
      mode: "light",
      density: "spacious",
      fontSize: "sm",
      motion: "full",
      palette: null,
    });
    const s = useStore.getState();
    expect(s.mode).toBe("light");
    expect(s.density).toBe("spacious");
    expect(s.fontSize).toBe("sm");
    expect(s.palette).toBeNull();
  });

  it("accepts custom initial seed", () => {
    const customStore = createLayoutPreferencesStore({
      storage: null,
      initial: { mode: "dark", palette: "ocean" },
    });
    const s = customStore.getState();
    expect(s.mode).toBe("dark");
    expect(s.palette).toBe("ocean");
    // Unspecified fields fall back to defaults.
    expect(s.density).toBe(defaultLayoutPreferences.density);
  });
});
