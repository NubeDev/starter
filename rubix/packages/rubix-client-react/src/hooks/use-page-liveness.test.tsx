// Tests for `usePageLiveness`. Drives the hook against a mock
// `EventSource`, asserts client-side filtering by `page_id` and the
// `changeToken` / `latestRevisionId` semantics defined in scope 11 §B1.

import { describe, expect, it } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { usePageLiveness } from "./use-page-liveness.js";
import { makeHarness } from "./test-harness.js";

class MockEventSource {
  static instances: MockEventSource[] = [];
  static reset(): void {
    MockEventSource.instances = [];
  }
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  closed = false;

  constructor(public url: string, public init?: EventSourceInit) {
    MockEventSource.instances.push(this);
  }
  close() {
    this.closed = true;
  }
  emit(frame: unknown) {
    this.onmessage?.({ data: JSON.stringify(frame) } as MessageEvent);
  }
}

describe("usePageLiveness", () => {
  it("subscribes to the shared dashboard-events route", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    renderHook(
      () =>
        usePageLiveness("dashboard.x", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    expect(MockEventSource.instances[0]!.url).toContain(
      "/api/v1/dashboards/events",
    );
  });

  it("seeds latestRevisionId from snapshot without bumping changeToken", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        usePageLiveness("dashboard.x", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({
        kind: "snapshot",
        items: [
          { page_id: "dashboard.other", title: "Other", revision_id: "r-o" },
          { page_id: "dashboard.x", title: "X", revision_id: "rev-seed" },
        ],
      }),
    );
    await waitFor(() =>
      expect(result.current.latestRevisionId).toBe("rev-seed"),
    );
    expect(result.current.changeToken).toBe(0);
  });

  it("bumps changeToken and updates latestRevisionId on a matching updated frame", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        usePageLiveness("dashboard.x", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({
        kind: "updated",
        page_id: "dashboard.x",
        revision_id: "rev-2",
        tenant_id: "t1",
        actor_kind: "ai",
      }),
    );
    await waitFor(() => expect(result.current.changeToken).toBe(1));
    expect(result.current.latestRevisionId).toBe("rev-2");
    expect(result.current.actorKind).toBe("ai");
  });

  it("ignores frames for a different page_id", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        usePageLiveness("dashboard.x", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({
        kind: "updated",
        page_id: "dashboard.someone-else",
        revision_id: "rev-q",
        tenant_id: "t1",
      }),
    );
    // Give react a tick — but token should stay 0.
    await new Promise((r) => setTimeout(r, 5));
    expect(result.current.changeToken).toBe(0);
    expect(result.current.latestRevisionId).toBeUndefined();
  });

  it("bumps changeToken on a matching deleted frame", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        usePageLiveness("dashboard.x", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({ kind: "deleted", page_id: "dashboard.x", tenant_id: "t1" }),
    );
    await waitFor(() => expect(result.current.changeToken).toBe(1));
  });
});
