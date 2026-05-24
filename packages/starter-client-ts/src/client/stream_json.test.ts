// Tests for the `streamJson` SSE primitive. Exercises both transport
// paths: the `fetch` + `ReadableStream` fallback (default in Node)
// and the `EventSource` path (via a minimal polyfill injected through
// `opts.eventSourceCtor`).

import { describe, expect, it, vi } from "vitest";

import { StarterClient } from "./client.js";
import { streamJson } from "./stream_json.js";

function sseResponse(frames: string[]): Response {
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const enc = new TextEncoder();
      for (const f of frames) controller.enqueue(enc.encode(f));
      controller.close();
    },
  });
  return new Response(stream, {
    status: 200,
    headers: { "content-type": "text/event-stream" },
  });
}

describe("streamJson — fetch fallback", () => {
  it("parses data: frames into typed values", async () => {
    const fake: typeof fetch = async () =>
      sseResponse([`data: {"n":1}\n\n`, `: heartbeat\ndata: {"n":2}\n\n`, `data: {"n":3}\n\n`]);
    const client = new StarterClient({ baseUrl: "http://t", fetch: fake });
    const ctrl = new AbortController();
    const out: number[] = [];
    for await (const v of streamJson<{ n: number }>(client, "/sse", {
      signal: ctrl.signal,
      forceFetch: true,
    })) {
      out.push(v.n);
      if (out.length === 3) {
        ctrl.abort();
        break;
      }
    }
    expect(out).toEqual([1, 2, 3]);
  });

  it("reconnects with backoff and invokes onReconnecting", async () => {
    let calls = 0;
    const fake: typeof fetch = async () => {
      calls += 1;
      if (calls === 1) return new Response("nope", { status: 500 });
      return sseResponse([`data: {"ok":true}\n\n`]);
    };
    const client = new StarterClient({ baseUrl: "http://t", fetch: fake });
    const ctrl = new AbortController();
    const onReconnecting = vi.fn((_attempt: number, _delay: number) => {
      // unblock the backoff sleep immediately so the test runs fast
      ctrl.abort();
    });
    const it = streamJson<{ ok: true }>(client, "/sse", {
      signal: ctrl.signal,
      forceFetch: true,
      onReconnecting,
    })[Symbol.asyncIterator]();
    const r = await it.next();
    expect(r.done).toBe(true);
    expect(onReconnecting).toHaveBeenCalledTimes(1);
    const [attempt, delay] = onReconnecting.mock.calls[0]!;
    expect(attempt).toBe(1);
    expect(delay).toBeGreaterThan(0);
    expect(delay).toBeLessThanOrEqual(1100);
  });

  it("returns cleanly when the signal is aborted mid-stream", async () => {
    const fake: typeof fetch = async () => sseResponse([`data: {"n":1}\n\n`]);
    const client = new StarterClient({ baseUrl: "http://t", fetch: fake });
    const ctrl = new AbortController();
    const collected: number[] = [];
    for await (const v of streamJson<{ n: number }>(client, "/sse", {
      signal: ctrl.signal,
      forceFetch: true,
    })) {
      collected.push(v.n);
      ctrl.abort();
    }
    expect(collected).toEqual([1]);
  });
});

// Minimal EventSource polyfill — just enough to drive `streamJson`.
class FakeEventSource {
  static instances: FakeEventSource[] = [];
  url: string;
  withCredentials: boolean;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  closed = false;
  constructor(url: string, init?: { withCredentials?: boolean }) {
    this.url = url;
    this.withCredentials = init?.withCredentials ?? false;
    FakeEventSource.instances.push(this);
  }
  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }
  fail() {
    this.onerror?.({} as Event);
  }
  close() {
    this.closed = true;
  }
}

describe("streamJson — EventSource path", () => {
  it("uses the injected EventSource ctor with withCredentials", async () => {
    FakeEventSource.instances = [];
    const client = new StarterClient({ baseUrl: "http://t", fetch: globalThis.fetch });
    const ctrl = new AbortController();
    const iter = streamJson<{ n: number }>(client, "/sse", {
      signal: ctrl.signal,
      eventSourceCtor: FakeEventSource as unknown as typeof EventSource,
    })[Symbol.asyncIterator]();

    // Wait a tick so the EventSource is constructed.
    await Promise.resolve();
    const es = FakeEventSource.instances[0]!;
    expect(es.url).toBe("http://t/sse");
    expect(es.withCredentials).toBe(true);

    es.emit({ n: 7 });
    const r = await iter.next();
    expect(r).toEqual({ value: { n: 7 }, done: false });

    ctrl.abort();
    expect(es.closed).toBe(true);
  });

  it("reconnects on EventSource error", async () => {
    FakeEventSource.instances = [];
    const client = new StarterClient({ baseUrl: "http://t", fetch: globalThis.fetch });
    const ctrl = new AbortController();
    const onReconnecting = vi.fn((_a: number, _d: number) => {
      ctrl.abort();
    });
    const iter = streamJson<unknown>(client, "/sse", {
      signal: ctrl.signal,
      eventSourceCtor: FakeEventSource as unknown as typeof EventSource,
      onReconnecting,
    })[Symbol.asyncIterator]();

    await Promise.resolve();
    FakeEventSource.instances[0]!.fail();

    const r = await iter.next();
    expect(r.done).toBe(true);
    expect(onReconnecting).toHaveBeenCalledTimes(1);
  });
});
