import { describe, expect, it } from "vitest";

import type { Actor, Op } from "@/api/types";
import { actorLabel, opLabel } from "@/features/audit/actorLabel";

// The audit row's "who/what" labels must read in domain terms for every actor
// and op variant — a user by subject, an agent by model, a system action
// unattributed; and the custom-op object variant by its name, not "[object]".
describe("audit labels", () => {
  it("labels a user actor by subject", () => {
    const actor: Actor = { kind: "user", subject: "alice" };
    expect(actorLabel(actor)).toBe("alice");
  });

  it("labels an agent actor by model", () => {
    const actor: Actor = { kind: "agent", model: "claude-x", run_id: "r1" };
    expect(actorLabel(actor)).toBe("agent · claude-x");
  });

  it("labels a system actor as system", () => {
    const actor: Actor = { kind: "system" };
    expect(actorLabel(actor)).toBe("system");
  });

  it("renders a string op verbatim", () => {
    expect(opLabel("update" as Op)).toBe("update");
  });

  it("renders a custom op by its name", () => {
    expect(opLabel({ custom: "publish" } as Op)).toBe("publish");
  });
});
