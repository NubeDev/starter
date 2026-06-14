import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";

// No global auto-cleanup is configured, so unmount between cases to keep the
// document free of stale `role="status"` nodes from a prior render.
afterEach(cleanup);

import { ProbeResult } from "@/features/datasources/DatasourceFormDialog";

// The pre-save probe outcome banner. A transport failure and a failed probe both
// read as "couldn't connect"; a successful probe reports latency. This is the
// only stateful presentation in the create form, so it carries the test.
describe("ProbeResult", () => {
  it("renders nothing while the probe is in flight", () => {
    const { container } = render(
      <ProbeResult pending failed={false} result={undefined} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders nothing before any probe has run", () => {
    const { container } = render(
      <ProbeResult pending={false} failed={false} result={undefined} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("reports success with latency", () => {
    const { getByRole } = render(
      <ProbeResult
        pending={false}
        failed={false}
        result={{ ok: true, latency_ms: 12 }}
      />,
    );
    expect(getByRole("status").textContent).toContain("12ms");
  });

  it("surfaces the sanitized message on a failed probe", () => {
    const { getByRole } = render(
      <ProbeResult
        pending={false}
        failed={false}
        result={{ ok: false, message: "password authentication failed" }}
      />,
    );
    expect(getByRole("status").textContent).toContain(
      "password authentication failed",
    );
  });

  it("reads a transport failure as a reach failure", () => {
    const { getByRole } = render(
      <ProbeResult pending={false} failed result={undefined} />,
    );
    expect(getByRole("status").textContent).toContain("Couldn't reach");
  });
});
