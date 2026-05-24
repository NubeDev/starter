// Smoke tests for `QueryProvider`. The interesting bit is the
// retry-skips-on-401/403 default — everything else is plumbing that
// TanStack Query already covers.

import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { useQuery } from "@tanstack/react-query";

import { StarterError } from "@nube/starter-client-ts";

import { QueryProvider } from "./query-provider.js";

function Probe(props: { fn: () => Promise<unknown> }) {
  const q = useQuery({ queryKey: ["t"], queryFn: props.fn, retryDelay: 1 });
  return <div data-testid="state">{q.status}</div>;
}

describe("QueryProvider", () => {
  it("does not retry on a StarterError 401", async () => {
    const fn = vi.fn().mockRejectedValue(new StarterError(401, "nope"));
    render(
      <QueryProvider>
        <Probe fn={fn} />
      </QueryProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("error"));
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("does not retry on a StarterError 403", async () => {
    const fn = vi.fn().mockRejectedValue(new StarterError(403, "nope"));
    render(
      <QueryProvider>
        <Probe fn={fn} />
      </QueryProvider>,
    );
    await waitFor(() => expect(screen.getByTestId("state").textContent).toBe("error"));
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("retries on a generic Error", async () => {
    const fn = vi.fn().mockRejectedValue(new Error("boom"));
    render(
      <QueryProvider>
        <Probe fn={fn} />
      </QueryProvider>,
    );
    await waitFor(() => expect(fn.mock.calls.length).toBeGreaterThan(1));
  });
});
