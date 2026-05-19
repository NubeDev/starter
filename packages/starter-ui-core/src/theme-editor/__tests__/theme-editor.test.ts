// Vitest coverage for the theme-editor surface called out in
// TODO.md Phase 9d: store undo/redo collapse window, CSS parse ↔
// generate round-trip, contrast tier boundaries, and the
// hex/rgb/hsl → oklch normalisation in `applyThemeToElement` (while
// preserving non-colour tokens).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { defaultThemeStyles } from "../defaults.js";
import { useThemeEditorStore } from "../store.js";
import { parseCssInput } from "../utils/parse-css-input.js";
import { generateCssString } from "../utils/generate-css.js";
import { getContrastTier } from "../utils/contrast-checker.js";
import { applyThemeToElement } from "../utils/apply-theme.js";

beforeEach(() => {
  // Hydrate the singleton store from defaults so each test sees a
  // pristine history ring + clean dirty flag.
  useThemeEditorStore.getState().hydrate(defaultThemeStyles, {
    nav_title: "",
    hide_features: [],
  });
});

describe("themeEditorStore undo/redo collapse", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00Z"));
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("collapses checkpoints inside the 500ms window into one", () => {
    const s = useThemeEditorStore.getState();
    expect(s.canUndo()).toBe(false);

    s.checkpoint();
    vi.advanceTimersByTime(100);
    s.checkpoint();
    vi.advanceTimersByTime(100);
    s.checkpoint();

    // Three rapid checkpoints → only one frame on the undo stack.
    s.undo();
    expect(useThemeEditorStore.getState().canUndo()).toBe(false);
  });

  it("treats checkpoints outside the window as distinct frames", () => {
    const s = useThemeEditorStore.getState();
    s.checkpoint();
    vi.advanceTimersByTime(600); // > COLLAPSE_MS (500)
    s.checkpoint();
    vi.advanceTimersByTime(600);
    s.checkpoint();

    // Each undo should consume one frame.
    s.undo();
    expect(useThemeEditorStore.getState().canUndo()).toBe(true);
    useThemeEditorStore.getState().undo();
    expect(useThemeEditorStore.getState().canUndo()).toBe(true);
    useThemeEditorStore.getState().undo();
    expect(useThemeEditorStore.getState().canUndo()).toBe(false);
  });

  it("redo replays an undone checkpoint", () => {
    const s = useThemeEditorStore.getState();
    s.checkpoint();
    vi.advanceTimersByTime(600);
    s.checkpoint();

    s.undo();
    expect(useThemeEditorStore.getState().canRedo()).toBe(true);
    useThemeEditorStore.getState().redo();
    expect(useThemeEditorStore.getState().canRedo()).toBe(false);
    expect(useThemeEditorStore.getState().canUndo()).toBe(true);
  });
});

describe("parseCssInput ↔ generateCssString round-trip", () => {
  it("round-trips a :root + .dark block losslessly", () => {
    const original = generateCssString({
      light: { primary: "oklch(0.5 0.2 250)", radius: "0.5rem" },
      dark: { primary: "oklch(0.7 0.2 250)" },
    });
    const parsed = parseCssInput(original);
    expect(parsed.light).toEqual({
      primary: "oklch(0.5 0.2 250)",
      radius: "0.5rem",
    });
    expect(parsed.dark).toEqual({ primary: "oklch(0.7 0.2 250)" });

    // Second hop through generate → parse should be identical.
    const regenerated = generateCssString({
      light: parsed.light ?? {},
      dark: parsed.dark ?? {},
    });
    expect(regenerated).toBe(original);
  });

  it("omits the dark block when dark has no declarations", () => {
    const css = generateCssString({
      light: { primary: "oklch(0.5 0.2 250)" },
      dark: {},
    });
    expect(css).not.toContain(".dark");
    expect(parseCssInput(css).dark).toBeUndefined();
  });

  it("ignores tokens outside :root / .dark blocks", () => {
    const parsed = parseCssInput(`
      :root { --primary: oklch(0.5 0.2 250); }
      body { color: red; }
      --orphan: 1px;
    `);
    expect(parsed.light).toEqual({ primary: "oklch(0.5 0.2 250)" });
    expect(parsed.dark).toBeUndefined();
  });
});

describe("getContrastTier boundary cases", () => {
  it("4.49 → fail (just below AA cutoff)", () => {
    expect(getContrastTier(4.49)).toBe("fail");
  });
  it("4.5 → AA (exactly on AA cutoff)", () => {
    expect(getContrastTier(4.5)).toBe("AA");
  });
  it("6.99 → AA (just below AAA cutoff)", () => {
    expect(getContrastTier(6.99)).toBe("AA");
  });
  it("7.0 → AAA (exactly on AAA cutoff)", () => {
    expect(getContrastTier(7.0)).toBe("AAA");
  });
  it("null → fail (unparseable input)", () => {
    expect(getContrastTier(null)).toBe("fail");
  });
});

describe("applyThemeToElement normalisation", () => {
  it("converts hex/rgb/hsl colour tokens to oklch(...)", () => {
    const el = document.createElement("div");
    applyThemeToElement(
      el,
      {
        // colour tokens — should be normalised to oklch(...)
        primary: "#ff0000",
        secondary: "rgb(0, 255, 0)",
        accent: "hsl(240, 100%, 50%)",
      },
      "light",
    );
    expect(el.style.getPropertyValue("--primary")).toMatch(/^oklch\(/);
    expect(el.style.getPropertyValue("--secondary")).toMatch(/^oklch\(/);
    expect(el.style.getPropertyValue("--accent")).toMatch(/^oklch\(/);
  });

  it("preserves non-colour tokens verbatim", () => {
    const el = document.createElement("div");
    applyThemeToElement(
      el,
      {
        radius: "0.75rem",
        "font-sans": '"Inter", system-ui, sans-serif',
        "shadow-blur": "12px",
        "shadow-offset-y": "4px",
      },
      "light",
    );
    expect(el.style.getPropertyValue("--radius")).toBe("0.75rem");
    expect(el.style.getPropertyValue("--font-sans")).toBe(
      '"Inter", system-ui, sans-serif',
    );
    expect(el.style.getPropertyValue("--shadow-blur")).toBe("12px");
    expect(el.style.getPropertyValue("--shadow-offset-y")).toBe("4px");
  });

  it("passes through oklch(...) inputs untouched (fast path)", () => {
    const el = document.createElement("div");
    applyThemeToElement(el, { primary: "oklch(0.58 0.22 257)" }, "light");
    expect(el.style.getPropertyValue("--primary")).toBe("oklch(0.58 0.22 257)");
  });

  it("toggles the .dark class on the element", () => {
    const el = document.createElement("div");
    applyThemeToElement(el, {}, "dark");
    expect(el.classList.contains("dark")).toBe(true);
    applyThemeToElement(el, {}, "light");
    expect(el.classList.contains("dark")).toBe(false);
  });

  it("removes properties for empty values", () => {
    const el = document.createElement("div");
    applyThemeToElement(el, { primary: "#ff0000" }, "light");
    expect(el.style.getPropertyValue("--primary")).not.toBe("");
    applyThemeToElement(el, { primary: "" }, "light");
    expect(el.style.getPropertyValue("--primary")).toBe("");
  });
});
