// Tests for `useEventStream`. Drives the hook against a custom
// mock `EventSource` so we can exercise the open / message /
// reconnect transitions deterministically.

import { describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";

import { StarterClient } from "@nube/starter-client-ts";

import { StarterClientProvider } from "../provider/starter-client-provider.js";
import { useEventStream } from "./use-event-stream.js";

class MockEventSource {
  static instances: MockEventSource[] = [];
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  closed = false;

  constructor(public url: string, public init?: EventSourceInit) {
    MockEventSource.instances.push(this);
  }
  close() {
    this.closed = true;
  }
  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }
}

function Probe(props: { ctor: typeof EventSource }) {
  const s = useEventStream<{ n: number }>("/sse", { eventSourceCtor: props.ctor });
  return (
    <div>
      <div data-testid="status">{s.status}</div>
      <div data-testid="data">{s.data ? String(s.data.n) : ""}</div>
      <button onClick={() => s.reconnect()}>reconnect</button>
    </div>
  );
}

function mount() {
  MockEventSource.instances = [];
  const client = new StarterClient({ baseUrl: "http://t", fetch: vi.fn() as unknown as typeof fetch });
  const utils = render(
    <StarterClientProvider client={client}>
      <Probe ctor={MockEventSource as unknown as typeof EventSource} />
    </StarterClientProvider>,
  );
  return utils;
}

describe("useEventStream", () => {
  it("starts in connecting, becomes open after a frame", async () => {
    mount();
    expect(screen.getByTestId("status").textContent).toBe("connecting");
    await waitFor(() => expect(MockEventSource.instances.length).toBe(1));
    act(() => MockEventSource.instances[0]!.emit({ n: 42 }));
    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("open"));
    expect(screen.getByTestId("data").textContent).toBe("42");
  });

  it("reconnect() tears down the previous EventSource and opens a new one", async () => {
    mount();
    await waitFor(() => expect(MockEventSource.instances.length).toBe(1));
    const first = MockEventSource.instances[0]!;
    act(() => first.emit({ n: 1 }));
    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("open"));

    act(() => {
      screen.getByText("reconnect").click();
    });
    await waitFor(() => expect(MockEventSource.instances.length).toBe(2));
    expect(first.closed).toBe(true);
  });

  it("cleans up on unmount", async () => {
    const { unmount } = mount();
    await waitFor(() => expect(MockEventSource.instances.length).toBe(1));
    const es = MockEventSource.instances[0]!;
    unmount();
    expect(es.closed).toBe(true);
  });
});
