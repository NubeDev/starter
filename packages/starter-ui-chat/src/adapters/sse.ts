import type {
  ChatAdapter,
  ChatMessage,
  ChatSendInput,
  ChatStreamDelta,
} from "../types/index.js";

export interface SseAdapterOptions {
  url: string;
  // Map a server-sent event payload to a ChatStreamDelta. Returning
  // undefined skips the event.
  parse?: (raw: string) => ChatStreamDelta | undefined;
  headers?: Record<string, string>;
  // Build the request body. Defaults to JSON { input, history }.
  body?: (input: ChatSendInput, history: ReadonlyArray<ChatMessage>) => BodyInit;
  fetchImpl?: typeof fetch;
}

const defaultParse = (raw: string): ChatStreamDelta | undefined => {
  if (!raw) return undefined;
  if (raw === "[DONE]") return { type: "done" };
  try {
    const obj = JSON.parse(raw) as ChatStreamDelta;
    if (obj && typeof obj === "object" && "type" in obj) return obj;
    return { type: "text", text: raw };
  } catch {
    return { type: "text", text: raw };
  }
};

// Minimal SSE adapter against a POST endpoint returning `text/event-stream`.
// Parses `data: ...` lines into ChatStreamDelta via `parse`.
export function createSseAdapter(opts: SseAdapterOptions): ChatAdapter {
  const fetchImpl = opts.fetchImpl ?? fetch;
  const parse = opts.parse ?? defaultParse;
  const buildBody =
    opts.body ??
    ((input: ChatSendInput, history: ReadonlyArray<ChatMessage>) =>
      JSON.stringify({ input, history }));

  return {
    async *send(input, history, signal) {
      const res = await fetchImpl(opts.url, {
        method: "POST",
        signal,
        headers: {
          "content-type": "application/json",
          accept: "text/event-stream",
          ...opts.headers,
        },
        body: buildBody(input, history),
      });
      if (!res.ok || !res.body) {
        yield {
          type: "error",
          error: `HTTP ${res.status} ${res.statusText}`,
        };
        return;
      }
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = "";
      while (true) {
        if (signal.aborted) return;
        const { value, done } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        let idx: number;
        while ((idx = buf.indexOf("\n\n")) !== -1) {
          const chunk = buf.slice(0, idx);
          buf = buf.slice(idx + 2);
          for (const line of chunk.split("\n")) {
            if (!line.startsWith("data:")) continue;
            const data = line.slice(5).trimStart();
            const delta = parse(data);
            if (delta) yield delta;
            if (delta?.type === "done") return;
          }
        }
      }
    },
  };
}
