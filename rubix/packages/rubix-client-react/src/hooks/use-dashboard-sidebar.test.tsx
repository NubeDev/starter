// Tests for `useDashboardSidebar`. Drives the hook against a mock
// `EventSource`, feeds it the snapshot + delta frames the rubix-agent
// SSE route emits, and asserts the reducer reaches the right
// title-sorted item list.

import { describe, expect, it } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import {
  useDashboardSidebar,
  type DashboardSidebarFrame,
} from "./use-dashboard-sidebar.js";
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
  emit(frame: DashboardSidebarFrame) {
    this.onmessage?.({ data: JSON.stringify(frame) } as MessageEvent);
  }
}

describe("useDashboardSidebar", () => {
  it("subscribes to the dashboard events route", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    renderHook(
      () =>
        useDashboardSidebar({
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    expect(MockEventSource.instances[0]!.url).toContain(
      "/api/v1/dashboards/events",
    );
  });

  it("seeds the list from the snapshot frame and applies a created delta", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useDashboardSidebar({
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
          { page_id: "dashboard.zeta", title: "Zeta", revision_id: "r1" },
          { page_id: "dashboard.alpha", title: "Alpha", revision_id: "r2" },
        ],
      }),
    );
    await waitFor(() => expect(result.current.items).toHaveLength(2));
    // Title-sorted: Alpha before Zeta.
    expect(result.current.items.map((it) => it.page_id)).toEqual([
      "dashboard.alpha",
      "dashboard.zeta",
    ]);

    act(() =>
      es.emit({
        kind: "created",
        page_id: "dashboard.mango",
        title: "Mango",
        revision_id: "r3",
        tenant_id: "t1",
      }),
    );
    await waitFor(() => expect(result.current.items).toHaveLength(3));
    expect(result.current.items.map((it) => it.title)).toEqual([
      "Alpha",
      "Mango",
      "Zeta",
    ]);
  });

  it("updates a title in place and drops on deleted", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useDashboardSidebar({
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({
        kind: "snapshot",
        items: [{ page_id: "dashboard.x", title: "X", revision_id: "r1" }],
      }),
    );
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() =>
      es.emit({
        kind: "updated",
        page_id: "dashboard.x",
        title: "X renamed",
        revision_id: "r2",
        tenant_id: "t1",
      }),
    );
    await waitFor(() =>
      expect(result.current.items[0]?.title).toBe("X renamed"),
    );
    expect(result.current.items[0]?.revision_id).toBe("r2");

    act(() => es.emit({ kind: "deleted", page_id: "dashboard.x", tenant_id: "t1" }));
    await waitFor(() => expect(result.current.items).toHaveLength(0));
  });

  it("snapshot on reconnect replaces the prior list", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useDashboardSidebar({
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    act(() =>
      MockEventSource.instances[0]!.emit({
        kind: "snapshot",
        items: [{ page_id: "dashboard.gone", title: "Gone", revision_id: "r1" }],
      }),
    );
    await waitFor(() => expect(result.current.items).toHaveLength(1));

    act(() => result.current.reconnect());
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(2));
    act(() =>
      MockEventSource.instances[1]!.emit({
        kind: "snapshot",
        items: [{ page_id: "dashboard.fresh", title: "Fresh", revision_id: "r9" }],
      }),
    );
    await waitFor(() =>
      expect(result.current.items.map((it) => it.page_id)).toEqual([
        "dashboard.fresh",
      ]),
    );
  });
});
