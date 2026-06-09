import { describe, expect, it } from "vitest";

import type { MeResponse } from "@/api/types";
import { can } from "@/auth/can";

// `can` is the pure authorization core `useCan` wraps. Tested directly so
// the gate logic is pinned independent of React. The principal shape is
// the real `MeResponse` (role + scopes + teams) — typed test input, not
// fabricated app data (F0).
const principal = (over: Partial<MeResponse>): MeResponse => ({
  subject: "u1",
  role: "reader",
  scopes: [],
  teams: [],
  tenant_id: "t1",
  ...over,
});

describe("can", () => {
  it("denies everything when there is no principal", () => {
    expect(can(null, "read")).toBe(false);
    expect(can(undefined, "write")).toBe(false);
  });

  it("admin can do anything", () => {
    const p = principal({ role: "admin" });
    expect(can(p, "read")).toBe(true);
    expect(can(p, "write")).toBe(true);
    expect(can(p, "admin")).toBe(true);
  });

  it("writer can read and write but not admin", () => {
    const p = principal({ role: "writer" });
    expect(can(p, "read")).toBe(true);
    expect(can(p, "write")).toBe(true);
    expect(can(p, "admin")).toBe(false);
  });

  it("reader can only read", () => {
    const p = principal({ role: "reader" });
    expect(can(p, "read")).toBe(true);
    expect(can(p, "write")).toBe(false);
  });

  it("an explicit scope grants its action regardless of role", () => {
    const p = principal({ role: "reader", scopes: ["datasources:write"] });
    expect(can(p, "write", "datasources:write")).toBe(true);
    // …but only the named scope, not a blanket write.
    expect(can(p, "write")).toBe(false);
  });

  it("membership of a team can be checked", () => {
    const p = principal({ teams: ["ops", "ingest"] });
    expect(can(p, "read", undefined, "ops")).toBe(true);
    expect(can(p, "read", undefined, "billing")).toBe(false);
  });
});
