// `streamJson` — Server-Sent Events primitive returning an
// `AsyncIterable<T>` of parsed JSON payloads. Used by higher-level
// React hooks (`@nube/starter-client-react`) to subscribe to live
// server feeds (e.g. extension status, flow runs).
//
// Two transports, picked at call time:
//
//   1. `EventSource` — preferred in browsers. Constructed with
//      `withCredentials: true` so the auth cookie is sent. Reconnect
//      handled by the browser, but we also wrap our own backoff loop
//      around it so we get parity with the fetch fallback and so the
//      consumer sees a clean async-iterator close on abort.
//   2. `fetch` + `ReadableStream` — node, tests, and browsers without
//      EventSource. Parses the `data: …\n\n` SSE frame format.
//
// Reconnect: exponential backoff (base 1s, cap 30s, ±10% jitter). On
// each retry we invoke `opts.onReconnecting(attempt, delayMs)` so
// consumers can surface a "reconnecting…" UI. Abort via
// `opts.signal` stops the iterator cleanly without throwing.
//
// CSRF: SSE uses a plain GET; no double-submit token is required per
// starter-server's cookie-only contract for safe methods.

import type { StarterClient } from "./client.js";

export interface StreamJsonOptions {
  /** Abort signal. When aborted, the iterator returns cleanly. */
  signal?: AbortSignal;
  /** Notified before each reconnect attempt (attempt is 1-indexed). */
  onReconnecting?: (attempt: number, delayMs: number) => void;
  /** Force the fetch fallback even when `EventSource` is available. */
  forceFetch?: boolean;
  /** Inject an `EventSource` constructor (tests / polyfills). */
  eventSourceCtor?: typeof EventSource;
}

const BACKOFF_BASE_MS = 1000;
const BACKOFF_CAP_MS = 30_000;

function backoffDelay(attempt: number): number {
  const exp = Math.min(BACKOFF_CAP_MS, BACKOFF_BASE_MS * 2 ** (attempt - 1));
  const jitter = exp * 0.1 * (Math.random() * 2 - 1);
  return Math.max(0, Math.round(exp + jitter));
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal?.aborted) return resolve();
    const t = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    const onAbort = () => {
      clearTimeout(t);
      resolve();
    };
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

export function streamJson<T>(
  client: StarterClient,
  path: string,
  opts: StreamJsonOptions = {},
): AsyncIterable<T> {
  const url = `${client.baseUrl}${path}`;
  const ctor =
    opts.eventSourceCtor ??
    (opts.forceFetch ? undefined : (globalThis as { EventSource?: typeof EventSource }).EventSource);

  return {
    [Symbol.asyncIterator](): AsyncIterator<T> {
      return ctor ? eventSourceIterator<T>(ctor, url, opts) : fetchIterator<T>(client, url, opts);
    },
  };
}

function eventSourceIterator<T>(
  Ctor: typeof EventSource,
  url: string,
  opts: StreamJsonOptions,
): AsyncIterator<T> {
  let es: EventSource | undefined;
  let attempt = 0;
  const queue: T[] = [];
  let waiter: ((v: IteratorResult<T>) => void) | undefined;
  let done = false;

  const push = (v: T) => {
    if (waiter) {
      const w = waiter;
      waiter = undefined;
      w({ value: v, done: false });
    } else queue.push(v);
  };
  const close = () => {
    done = true;
    es?.close();
    if (waiter) {
      const w = waiter;
      waiter = undefined;
      w({ value: undefined, done: true });
    }
  };

  const connect = () => {
    es = new Ctor(url, { withCredentials: true });
    es.onmessage = (ev: MessageEvent) => {
      attempt = 0;
      try {
        push(JSON.parse(String(ev.data)) as T);
      } catch {
        // ignore unparseable frames
      }
    };
    es.onerror = async () => {
      es?.close();
      if (done) return;
      attempt += 1;
      const delay = backoffDelay(attempt);
      opts.onReconnecting?.(attempt, delay);
      await sleep(delay, opts.signal);
      if (done || opts.signal?.aborted) return close();
      connect();
    };
  };

  opts.signal?.addEventListener("abort", close, { once: true });
  connect();

  return {
    next(): Promise<IteratorResult<T>> {
      if (queue.length > 0) return Promise.resolve({ value: queue.shift()!, done: false });
      if (done) return Promise.resolve({ value: undefined, done: true });
      return new Promise((resolve) => (waiter = resolve));
    },
    async return(): Promise<IteratorResult<T>> {
      close();
      return { value: undefined, done: true };
    },
  };
}

async function* fetchIterator<T>(
  client: StarterClient,
  url: string,
  opts: StreamJsonOptions,
): AsyncGenerator<T> {
  let attempt = 0;
  while (!opts.signal?.aborted) {
    try {
      const res = await client.fetch(url, {
        method: "GET",
        credentials: "include",
        headers: { ...client.headers, accept: "text/event-stream" },
        signal: opts.signal,
      });
      if (!res.ok || !res.body) throw new Error(`SSE HTTP ${res.status}`);
      attempt = 0;
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = "";
      while (!opts.signal?.aborted) {
        const { value, done } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        let idx: number;
        while ((idx = buf.indexOf("\n\n")) !== -1) {
          const frame = buf.slice(0, idx);
          buf = buf.slice(idx + 2);
          const data = frame
            .split("\n")
            .filter((l) => l.startsWith("data:"))
            .map((l) => l.slice(5).replace(/^ /, ""))
            .join("\n");
          if (!data) continue;
          try {
            yield JSON.parse(data) as T;
          } catch {
            // ignore unparseable frame
          }
        }
      }
    } catch (err) {
      if (opts.signal?.aborted) return;
      if ((err as { name?: string }).name === "AbortError") return;
    }
    if (opts.signal?.aborted) return;
    attempt += 1;
    const delay = backoffDelay(attempt);
    opts.onReconnecting?.(attempt, delay);
    await sleep(delay, opts.signal);
  }
}
