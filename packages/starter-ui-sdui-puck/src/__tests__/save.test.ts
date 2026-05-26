// `makeRubixSaveTransport` translation tests — guarantees the rubix
// frontend route (§B5) can wire `client.dashboardUpdate` directly
// without re-implementing the 409 → conflict mapping.

import { describe, expect, it, vi } from "vitest";

import { IR_VERSION } from "../adapter.js";
import {
  makeRubixSaveTransport,
  type DashboardUpdateLikeClient,
  type PuckSaveRequest,
} from "../save.js";

function request(overrides: Partial<PuckSaveRequest> = {}): PuckSaveRequest {
  return {
    pageRef: "dashboard.data-flow-site-a",
    body: {
      ir_version: IR_VERSION,
      root: { type: "page", children: [] },
    },
    expectedRevisionId: "rev-1",
    ...overrides,
  };
}

describe("makeRubixSaveTransport", () => {
  it("returns saved on success and forwards the new revision id", async () => {
    const client: DashboardUpdateLikeClient = {
      dashboardUpdate: vi.fn().mockResolvedValue({
        summary: { code: "ok" },
        page_id: "dashboard.data-flow-site-a",
        revision_id: "rev-2",
      }),
    };
    const save = makeRubixSaveTransport(client);
    const out = await save(request());
    expect(out).toEqual({ kind: "saved", revisionId: "rev-2" });
    expect(client.dashboardUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        page_id: "dashboard.data-flow-site-a",
        expected_revision_id: "rev-1",
      }),
    );
  });

  it("maps HTTP 409 onto a conflict outcome with the server's revision", async () => {
    const err = Object.assign(new Error("conflict"), {
      status: 409,
      problem: {
        title:
          "rubix.dashboard.update.conflict: page_id=dashboard.data-flow-site-a current_revision_id=rev-server",
      },
    });
    const client: DashboardUpdateLikeClient = {
      dashboardUpdate: vi.fn().mockRejectedValue(err),
    };
    const save = makeRubixSaveTransport(client);
    const out = await save(request());
    expect(out.kind).toBe("conflict");
    if (out.kind === "conflict") {
      expect(out.currentRevisionId).toBe("rev-server");
    }
  });

  it("maps other errors onto a generic error outcome", async () => {
    const err = Object.assign(new Error("boom"), { status: 500 });
    const client: DashboardUpdateLikeClient = {
      dashboardUpdate: vi.fn().mockRejectedValue(err),
    };
    const save = makeRubixSaveTransport(client);
    const out = await save(request());
    expect(out).toEqual({ kind: "error", message: "boom" });
  });

  it("refuses to save without an expected_revision_id", async () => {
    const client: DashboardUpdateLikeClient = {
      dashboardUpdate: vi.fn(),
    };
    const save = makeRubixSaveTransport(client);
    const out = await save(request({ expectedRevisionId: undefined }));
    expect(out.kind).toBe("error");
    expect(client.dashboardUpdate).not.toHaveBeenCalled();
  });
});
