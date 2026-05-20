import type {
  ChatAdapter,
  ChatMessage,
  ChatSendInput,
  ChatStreamDelta,
} from "../types/index.js";

// A toy adapter for demos and tests — streams the user message back as
// the assistant, one token at a time. Useful before wiring real
// transport.
export function createEchoAdapter(opts: { delayMs?: number } = {}): ChatAdapter {
  const { delayMs = 25 } = opts;
  return {
    async *send(
      input: ChatSendInput,
      _history: ReadonlyArray<ChatMessage>,
      signal: AbortSignal,
    ): AsyncIterable<ChatStreamDelta> {
      const tokens = input.text.split(/(\s+)/);
      for (const t of tokens) {
        if (signal.aborted) return;
        await new Promise((r) => setTimeout(r, delayMs));
        yield { type: "text", text: t };
      }
      yield { type: "done" };
    },
  };
}
