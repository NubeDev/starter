// Tests for `useFlowEvents`. Drives `useEventStream` through a
// mock `EventSource` and asserts: the SSE path is per-flow, frames
// accumulate into the buffer (capped at `bufferSize`), `runOverlay`
// aggregates emissions into `{ nodes: "ok", slotValues }`, and
// `reconnect()` clears state and opens a new EventSource.

import { describe, expect, it } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import {
  flowEventsPath,
  useFlowEvents,
  type NodeSlotValue,
} from "./flow-events.js";
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
  emit(data: NodeSlotValue) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }
}

const emit = (
  node: string,
  slot: string,
  value: unknown,
  run = "01J0RUNAAAAAAAAAAAAAAAAAAA",
): NodeSlotValue => ({ run, node, slot, value });

describe("flowEventsPath", () => {
  it("encodes the flow id and lands under /api/v1/flows", () => {
    expect(flowEventsPath("dev.starter.echo")).toBe(
      "/api/v1/flows/dev.starter.echo/events",
    );
    expect(flowEventsPath("a/b")).toBe("/api/v1/flows/a%2Fb/events");
  });
});

describe("useFlowEvents", () => {
  it("subscribes to the per-flow SSE path and accumulates frames", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useFlowEvents("dev.starter.echo", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );

    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    expect(es.url).toContain("/api/v1/flows/dev.starter.echo/events");

    act(() => es.emit(emit("dev.starter.counter", "count", 1)));
    act(() => es.emit(emit("dev.starter.counter", "count", 2)));

    await waitFor(() => expect(result.current.events).toHaveLength(2));
    expect(result.current.latest?.value).toBe(2);
    expect(result.current.status).toBe("open");
  });

  it("aggregates emissions into runOverlay (nodes=ok, slotValues=latest)", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useFlowEvents("dev.starter.echo", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;

    act(() => es.emit(emit("dev.starter.counter", "count", 1)));
    act(() => es.emit(emit("dev.starter.counter", "count", 2)));
    act(() => es.emit(emit("dev.starter.log", "out", "hello")));

    await waitFor(() => expect(result.current.events).toHaveLength(3));
    expect(result.current.runOverlay.nodes).toEqual({
      "dev.starter.counter": "ok",
      "dev.starter.log": "ok",
    });
    expect(result.current.runOverlay.slotValues).toEqual({
      "dev.starter.counter": { count: 2 },
      "dev.starter.log": { out: "hello" },
    });
  });

  it("caps the buffer at bufferSize", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useFlowEvents("dev.starter.echo", {
          bufferSize: 2,
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() => es.emit(emit("dev.starter.counter", "count", 1)));
    act(() => es.emit(emit("dev.starter.counter", "count", 2)));
    act(() => es.emit(emit("dev.starter.counter", "count", 3)));
    await waitFor(() => expect(result.current.events).toHaveLength(2));
    expect(result.current.events.map((e) => e.value)).toEqual([2, 3]);
  });

  it("reconnect() clears events + overlay and opens a new EventSource", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useFlowEvents("dev.starter.echo", {
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    act(() =>
      MockEventSource.instances[0]!.emit(
        emit("dev.starter.counter", "count", 7),
      ),
    );
    await waitFor(() => expect(result.current.events).toHaveLength(1));

    act(() => result.current.reconnect());
    await waitFor(() =>
      expect(MockEventSource.instances.length).toBeGreaterThanOrEqual(2),
    );
    expect(result.current.events).toHaveLength(0);
    expect(result.current.runOverlay.nodes).toEqual({});
    expect(result.current.runOverlay.slotValues).toEqual({});
  });
});
