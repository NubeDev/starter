// Tests for `useExtensionEvents`. Uses a mock `EventSource` to
// drive `useEventStream` deterministically, then asserts the
// accumulated `events` buffer, `latest`, status transitions, and
// that `reconnect()` clears the buffer and reopens.

import { describe, expect, it } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { useExtensionEvents, type ExtensionEvent } from "./use-extension-events.js";
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
  emit(data: ExtensionEvent) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }
}

const lifecycle = (state: string, at_ms = 1): ExtensionEvent => ({
  kind: "lifecycle",
  extension_id: "ext-a",
  state,
  at_ms,
});

describe("useExtensionEvents", () => {
  it("accumulates frames into the events buffer and exposes latest", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useExtensionEvents({
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );

    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    expect(es.url).toContain("/api/v1/extensions/events");

    act(() => es.emit(lifecycle("starting", 1)));
    act(() => es.emit(lifecycle("running", 2)));

    await waitFor(() => expect(result.current.events).toHaveLength(2));
    expect(result.current.latest?.kind).toBe("lifecycle");
    expect(result.current.status).toBe("open");
  });

  it("caps the buffer at bufferSize", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useExtensionEvents({
          bufferSize: 2,
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() => es.emit(lifecycle("a", 1)));
    act(() => es.emit(lifecycle("b", 2)));
    act(() => es.emit(lifecycle("c", 3)));
    await waitFor(() => expect(result.current.events).toHaveLength(2));
    expect(result.current.events.map((e) => (e as { state: string }).state)).toEqual([
      "b",
      "c",
    ]);
  });

  it("reconnect() clears the buffer and opens a new EventSource", async () => {
    MockEventSource.reset();
    const { Wrapper } = makeHarness(() => new Response("{}", { status: 200 }));
    const { result } = renderHook(
      () =>
        useExtensionEvents({
          eventSourceCtor: MockEventSource as unknown as typeof EventSource,
        }),
      { wrapper: Wrapper },
    );
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    act(() => MockEventSource.instances[0]!.emit(lifecycle("running", 1)));
    await waitFor(() => expect(result.current.events).toHaveLength(1));

    act(() => result.current.reconnect());
    await waitFor(() => expect(MockEventSource.instances.length).toBeGreaterThanOrEqual(2));
    expect(result.current.events).toHaveLength(0);
  });
});
