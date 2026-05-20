// Pure-function tests for the D-NP.6 locale fallback resolver.

import { describe, expect, it } from "vitest";

import { I18N_FALLBACK_LANGUAGE, resolveLocale } from "./locale-fallback.js";

describe("resolveLocale", () => {
  const manifest = {
    en: "fp-en",
    es: "fp-es",
    pt: "fp-pt",
    "zh-Hant": "fp-zh-hant",
  } as const;

  it("returns the requested tag when it has a catalog (no fallback)", () => {
    const r = resolveLocale("es", manifest);
    expect(r).not.toBeNull();
    expect(r!.picked).toBe("es");
    expect(r!.fingerprint).toBe("fp-es");
    expect(r!.fallbackUsed).toBe(false);
    expect(r!.chain).toEqual(["es"]);
  });

  it("left-truncates es-MX → es", () => {
    const r = resolveLocale("es-MX", manifest);
    expect(r!.picked).toBe("es");
    expect(r!.fingerprint).toBe("fp-es");
    expect(r!.fallbackUsed).toBe(true);
    expect(r!.chain).toEqual(["es-MX", "es"]);
  });

  it("walks pt-BR → pt", () => {
    const r = resolveLocale("pt-BR", manifest);
    expect(r!.picked).toBe("pt");
    expect(r!.chain).toEqual(["pt-BR", "pt"]);
    expect(r!.fallbackUsed).toBe(true);
  });

  it("falls through to the en floor when no segment matches", () => {
    const r = resolveLocale("fr-CA", manifest);
    expect(r!.picked).toBe("en");
    expect(r!.fingerprint).toBe("fp-en");
    expect(r!.fallbackUsed).toBe(true);
    expect(r!.chain).toEqual(["fr-CA", "fr", "en"]);
  });

  it("falls through to en for an entirely unknown tag", () => {
    const r = resolveLocale("xx", manifest);
    expect(r!.picked).toBe("en");
    expect(r!.chain).toEqual(["xx", "en"]);
  });

  it("treats undefined as 'ask for the floor'", () => {
    const r = resolveLocale(undefined, manifest);
    expect(r!.picked).toBe("en");
    expect(r!.fallbackUsed).toBe(false);
    expect(r!.chain).toEqual(["en"]);
  });

  it("returns null when even the en floor is absent", () => {
    expect(resolveLocale("es", { de: "fp-de" } as const)).toBeNull();
  });

  it("matches case-insensitively but returns the manifest's spelling", () => {
    const r = resolveLocale("ZH-hant", manifest);
    expect(r!.picked).toBe("zh-Hant");
    expect(r!.fingerprint).toBe("fp-zh-hant");
    expect(r!.fallbackUsed).toBe(false);
  });

  it("requesting en against a manifest with en is not a fallback", () => {
    const r = resolveLocale(I18N_FALLBACK_LANGUAGE, manifest);
    expect(r!.picked).toBe("en");
    expect(r!.fallbackUsed).toBe(false);
  });
});
