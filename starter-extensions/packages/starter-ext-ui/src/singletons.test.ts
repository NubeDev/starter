// Unit tests for the singleton-major utilities.

import { describe, expect, it } from "vitest";

import { matchingMajor, parseMajor } from "./singletons.js";

describe("parseMajor", () => {
  it("parses common semver shapes", () => {
    expect(parseMajor("18.3.1")).toBe(18);
    expect(parseMajor("18")).toBe(18);
    expect(parseMajor("18.3")).toBe(18);
    expect(parseMajor("^18.3.0")).toBe(18);
    expect(parseMajor("~18.3.0")).toBe(18);
    expect(parseMajor("19.0.0-rc.1")).toBe(19);
  });

  it("returns null for inputs without a leading number", () => {
    expect(parseMajor("")).toBeNull();
    expect(parseMajor("latest")).toBeNull();
    expect(parseMajor("x.y.z")).toBeNull();
  });
});

describe("matchingMajor", () => {
  it("accepts same-major across minor / patch / pre-release", () => {
    expect(matchingMajor("18.3.1", "18.0.0")).toBe(true);
    expect(matchingMajor("18.3.1", "18.99.99")).toBe(true);
    expect(matchingMajor("18.0.0-rc.1", "18.3.1")).toBe(true);
  });

  it("rejects different-major", () => {
    expect(matchingMajor("18.3.1", "19.0.0")).toBe(false);
    expect(matchingMajor("17.0.2", "18.0.0")).toBe(false);
  });

  it("rejects malformed inputs on either side", () => {
    expect(matchingMajor("garbage", "18.0.0")).toBe(false);
    expect(matchingMajor("18.0.0", "")).toBe(false);
  });
});
