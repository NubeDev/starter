import { describe, it, expect } from "vitest";
import { parseRefUids, readServiceRefUids } from "./scheduleService";
import type { Component, Property } from "../lib/engine-types";

const prop = (value: unknown): Property => ({ uid: 1, componentUid: 1, category: 1, value, statusFlags: 0 }) as Property;

describe("parseRefUids — tolerant of the three plausible ref encodings", () => {
  it("JSON number array", () => {
    expect(parseRefUids("[100623, 100636, 100659]")).toEqual([100623, 100636, 100659]);
  });
  it("JSON array of {uid} objects", () => {
    expect(parseRefUids('[{"uid":100623,"name":"a"},{"uid":100636}]')).toEqual([100623, 100636]);
  });
  it("comma-separated string", () => {
    expect(parseRefUids("100623,100636,100659")).toEqual([100623, 100636, 100659]);
  });
  it("space-separated string", () => {
    expect(parseRefUids("100623 100636")).toEqual([100623, 100636]);
  });
  it("already a JS array (pre-parsed)", () => {
    expect(parseRefUids([100623, 100636])).toEqual([100623, 100636]);
  });
  it("single number", () => {
    expect(parseRefUids(100623)).toEqual([100623]);
  });
  it("dedupes and drops non-positive / non-integer", () => {
    expect(parseRefUids("100623,100623,0,-5,1.5,abc,100636")).toEqual([100623, 100636]);
  });
  it("empty / null / garbage → []", () => {
    expect(parseRefUids("")).toEqual([]);
    expect(parseRefUids(null)).toEqual([]);
    expect(parseRefUids(undefined)).toEqual([]);
    expect(parseRefUids("{}")).toEqual([]);
  });
});

describe("readServiceRefUids — finds the ref-list prop without it being pinned", () => {
  const svc = (props: Record<string, unknown>): Component =>
    ({ uid: 9, name: "schedules", type: "NubeIO-schedule::schedule.service", path: "root/svc", parent: 1,
       properties: Object.fromEntries(Object.entries(props).map(([k, v]) => [k, prop(v)])) }) as Component;

  it("reads the canonical `schedules` prop", () => {
    expect(readServiceRefUids(svc({ schedules: "[100623,100636]", __facets: "" }))).toEqual([100623, 100636]);
  });
  it("falls back to `out`", () => {
    expect(readServiceRefUids(svc({ out: "100623,100636", __facets: "" }))).toEqual([100623, 100636]);
  });
  it("last-resort scans an unknown prop name holding a uid list", () => {
    expect(readServiceRefUids(svc({ members: "[100623,100636]", __facets: "" }))).toEqual([100623, 100636]);
  });
  it("ignores __facets and returns [] when nothing parses", () => {
    expect(readServiceRefUids(svc({ __facets: "xx", label: "hi" }))).toEqual([]);
  });
});
