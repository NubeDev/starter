import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  PALETTE_STORAGE_KEY,
  THEME_STORAGE_KEY,
  applyTheme,
  palettes,
  readStoredPalette,
  readStoredPreference,
  resolveMode,
  resolvePalette,
  theme,
} from "@/theme/theme";

describe("theme runtime", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("style");
    document.documentElement.classList.remove("dark");
  });
  afterEach(() => localStorage.clear());

  it("loads the JSON theme with both modes", () => {
    expect(theme.name).toBeTruthy();
    expect(theme.tokens.light.background).toBeTruthy();
    expect(theme.tokens.dark.background).toBeTruthy();
  });

  it("ships at least 3 palettes, each with light + dark tokens", () => {
    expect(palettes.length).toBeGreaterThanOrEqual(3);
    for (const p of palettes) {
      expect(p.id).toBeTruthy();
      expect(p.name).toBeTruthy();
      expect(p.light.primary).toBeTruthy();
      expect(p.dark.primary).toBeTruthy();
      expect(p.light.background).toBeTruthy();
      expect(p.dark.background).toBeTruthy();
    }
    // ids are unique — they're the persisted keys.
    const ids = palettes.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("the default palette resolves to a real palette", () => {
    expect(resolvePalette(theme.defaultPalette).id).toBe(theme.defaultPalette);
    // unknown id falls back, never throws
    expect(resolvePalette("nope").id).toBeTruthy();
  });

  it("applyTheme writes the selected palette's tokens and flips .dark", () => {
    const ocean = palettes.find((p) => p.id === "blue")!;
    applyTheme("dark", ocean.id);
    const root = document.documentElement;
    expect(root.classList.contains("dark")).toBe(true);
    expect(root.style.colorScheme).toBe("dark");
    expect(root.dataset.palette).toBe("blue");
    expect(root.style.getPropertyValue("--primary")).toBe(ocean.dark.primary);

    applyTheme("light", ocean.id);
    expect(root.classList.contains("dark")).toBe(false);
    expect(root.style.getPropertyValue("--primary")).toBe(ocean.light.primary);
  });

  it("switching palettes repaints --primary in the same mode", () => {
    const emerald = palettes.find((p) => p.id === "emerald")!;
    const violet = palettes.find((p) => p.id === "violet")!;
    const root = document.documentElement;

    applyTheme("dark", emerald.id);
    expect(root.style.getPropertyValue("--primary")).toBe(emerald.dark.primary);
    applyTheme("dark", violet.id);
    expect(root.style.getPropertyValue("--primary")).toBe(violet.dark.primary);
    expect(emerald.dark.primary).not.toBe(violet.dark.primary);
  });

  it("readStoredPreference honours a stored value, else the default", () => {
    expect(readStoredPreference()).toBe(theme.defaultMode);
    localStorage.setItem(THEME_STORAGE_KEY, "light");
    expect(readStoredPreference()).toBe("light");
    localStorage.setItem(THEME_STORAGE_KEY, "garbage");
    expect(readStoredPreference()).toBe(theme.defaultMode);
  });

  it("readStoredPalette honours a known id, else the default", () => {
    expect(readStoredPalette()).toBe(theme.defaultPalette);
    localStorage.setItem(PALETTE_STORAGE_KEY, "violet");
    expect(readStoredPalette()).toBe("violet");
    localStorage.setItem(PALETTE_STORAGE_KEY, "garbage");
    expect(readStoredPalette()).toBe(theme.defaultPalette);
  });

  it("resolveMode passes explicit modes through", () => {
    expect(resolveMode("light")).toBe("light");
    expect(resolveMode("dark")).toBe("dark");
  });
});
