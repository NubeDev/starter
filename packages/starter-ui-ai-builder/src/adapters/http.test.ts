// Unit tests for `createHttpBuilderAdapter` against in-memory
// `ReadableStream`s. Covers the 9 cases enumerated in
// `examples/flow-agent/PAGE-BUILDER-LIVE-FRONTEND.md` §4.5.

import { describe, expect, it, vi } from "vitest";

import type {
  BuilderAdapter,
  BuilderEvent,
} from "../types/index.js";
import { createHttpBuilderAdapter } from "./http.js";

/** Helper: typed `vi.fn` factory that mirrors the `fetch` signature
 *  the adapter expects. Lets test bodies destructure `mock.calls`
 *  without `as unknown as …` gymnastics. */
function mockFetch(impl: (url: string, init: RequestInit) => Promise<Response>) {
  return vi.fn(impl as (url: string, init: RequestInit) => Promise<Response>);
}

function encode(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function streamFromChunks(chunks: Array<Uint8Array | string>): ReadableStream<Uint8Array> {
  const bytes = chunks.map((c) => (typeof c === "string" ? encode(c) : c));
  return new ReadableStream<Uint8Array>({
    start(controller) {
      for (const b of bytes) controller.enqueue(b);
      controller.close();
    },
  });
}

function sseFrame(ev: BuilderEvent): string {
  return `data: ${JSON.stringify(ev)}\n\n`;
}

function okSseResponse(body: ReadableStream<Uint8Array>): Response {
  return new Response(body, {
    status: 200,
    headers: { "content-type": "text/event-stream" },
  });
}

async function collect(
  adapter: BuilderAdapter,
  signal?: AbortSignal,
): Promise<BuilderEvent[]> {
  const events: BuilderEvent[] = [];
  const ctrl = signal ? null : new AbortController();
  for await (const ev of adapter.send({ text: "hello" }, signal ?? ctrl!.signal)) {
    events.push(ev);
  }
  return events;
}

describe("createHttpBuilderAdapter", () => {
  it("happy path: yields thinking → writing → full-render → done in order", async () => {
    const tree = { ir_version: 5, root: { id: "r", type: "page" } };
    const fetchMock = mockFetch(async (_url, _init) =>
      okSseResponse(
        streamFromChunks([
          sseFrame({ type: "status", phase: "thinking", message: "Asking Claude…" }),
          sseFrame({ type: "status", phase: "writing" }),
          sseFrame({ type: "full-render", tree: tree as never }),
          sseFrame({ type: "status", phase: "done" }),
        ]),
      ),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toEqual([
      { type: "status", phase: "thinking", message: "Asking Claude…" },
      { type: "status", phase: "writing" },
      { type: "full-render", tree },
      { type: "status", phase: "done" },
    ]);
    expect(fetchMock).toHaveBeenCalledOnce();
    const call = (fetchMock.mock.calls as unknown as Array<[string, RequestInit]>)[0]!;
    expect(JSON.parse(String(call[1].body))).toEqual({
      prompt: "hello",
      provider: "claude",
    });
  });

  it("yields error frame mid-stream then ends", async () => {
    const fetchMock = mockFetch(async (_url, _init) =>
      okSseResponse(
        streamFromChunks([
          sseFrame({ type: "status", phase: "thinking" }),
          sseFrame({ type: "error", error: "boom" }),
        ]),
      ),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toEqual([
      { type: "status", phase: "thinking" },
      { type: "error", error: "boom" },
    ]);
  });

  it("synthesises an error event for malformed JSON, keeps reading", async () => {
    const fetchMock = mockFetch(async (_url, _init) =>
      okSseResponse(
        streamFromChunks([
          "data: {not json\n\n",
          sseFrame({ type: "status", phase: "done" }),
        ]),
      ),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toHaveLength(2);
    expect(out[0]?.type).toBe("error");
    expect((out[0] as { error: string }).error).toMatch(/malformed sse frame/);
    expect(out[1]).toEqual({ type: "status", phase: "done" });
  });

  it("reassembles a frame split across multiple chunks", async () => {
    const full = sseFrame({ type: "status", phase: "thinking" });
    const cut = Math.floor(full.length / 2);
    const fetchMock = mockFetch(async (_url, _init) =>
      okSseResponse(streamFromChunks([full.slice(0, cut), full.slice(cut)])),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toEqual([{ type: "status", phase: "thinking" }]);
  });

  it("aborts mid-stream: stops yielding after signal flips", async () => {
    const ctrl = new AbortController();
    const fetchMock = mockFetch(async (_url, init) => {
      const body = new ReadableStream<Uint8Array>({
        async pull(controller) {
          controller.enqueue(
            encode(sseFrame({ type: "status", phase: "thinking" })),
          );
          // Flip abort before enqueuing the next frame.
          ctrl.abort();
          controller.enqueue(
            encode(sseFrame({ type: "status", phase: "writing" })),
          );
          controller.close();
        },
      });
      // Mirror real `fetch` honouring the signal.
      init.signal?.addEventListener("abort", () => {
        try {
          body.cancel();
        } catch {
          /* ignore */
        }
      });
      return okSseResponse(body);
    });
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out: BuilderEvent[] = [];
    for await (const ev of adapter.send({ text: "hello" }, ctrl.signal)) {
      out.push(ev);
    }
    // Only the first frame should have made it through; the second
    // was yielded by the stream but the adapter must drop it post-abort.
    expect(out.length).toBeLessThanOrEqual(1);
    if (out.length === 1) {
      expect(out[0]).toEqual({ type: "status", phase: "thinking" });
    }
  });

  it("HTTP 503 with onUnavailable: delegates to fallback adapter", async () => {
    const fetchMock = mockFetch(async (_url, _init) =>
      new Response(
        JSON.stringify({ error: "provider unavailable", hint: "claude not on PATH" }),
        { status: 503, headers: { "content-type": "application/json" } },
      ),
    );
    const fallback: BuilderAdapter = {
      async *send() {
        yield { type: "status", phase: "thinking" };
        yield { type: "status", phase: "done" };
      },
    };
    const adapter = createHttpBuilderAdapter({
      url: "/api/x",
      fetch: fetchMock as never,
      onUnavailable: () => fallback,
    });
    const out = await collect(adapter);
    expect(out).toEqual([
      { type: "status", phase: "thinking" },
      { type: "status", phase: "done" },
    ]);
  });

  it("HTTP 503 without onUnavailable: single error event from body hint", async () => {
    const fetchMock = mockFetch(async (_url, _init) =>
      new Response(
        JSON.stringify({ error: "provider unavailable", hint: "claude missing" }),
        { status: 503, headers: { "content-type": "application/json" } },
      ),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toHaveLength(1);
    expect(out[0]?.type).toBe("error");
    expect((out[0] as { error: string }).error).toContain("provider unavailable");
    expect((out[0] as { error: string }).error).toContain("claude missing");
  });

  it("HTTP 500: yields one error event with status text", async () => {
    const fetchMock = mockFetch(async (_url, _init) =>
      new Response("kaboom", { status: 500, statusText: "Internal Server Error" }),
    );
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const out = await collect(adapter);
    expect(out).toHaveLength(1);
    expect(out[0]?.type).toBe("error");
  });

  it("already-aborted signal: yields nothing, never calls fetch", async () => {
    const fetchMock = mockFetch(async (_url, _init) => okSseResponse(streamFromChunks([])));
    const adapter = createHttpBuilderAdapter({ url: "/api/x", fetch: fetchMock as never });
    const ctrl = new AbortController();
    ctrl.abort();
    const out: BuilderEvent[] = [];
    for await (const ev of adapter.send({ text: "hello" }, ctrl.signal)) {
      out.push(ev);
    }
    expect(out).toEqual([]);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
