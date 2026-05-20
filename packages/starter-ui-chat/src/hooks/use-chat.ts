import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ChatAdapter,
  ChatMessage,
  ChatSendInput,
  ChatStatus,
} from "../types/index.js";
import { makeId } from "../lib/utils.js";

export interface UseChatOptions {
  adapter: ChatAdapter;
  initialMessages?: ChatMessage[];
  onError?: (err: unknown) => void;
  onFinish?: (assistant: ChatMessage) => void;
}

export interface UseChatReturn {
  messages: ChatMessage[];
  status: ChatStatus;
  error: string | null;
  send: (input: ChatSendInput | string) => Promise<void>;
  cancel: () => void;
  reset: (messages?: ChatMessage[]) => void;
  setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
}

// Headless chat state machine. The view layer just renders `messages`
// and calls `send`/`cancel`. Transport is the adapter's concern.
export function useChat(opts: UseChatOptions): UseChatReturn {
  const { adapter, initialMessages, onError, onFinish } = opts;
  const [messages, setMessages] = useState<ChatMessage[]>(
    () => initialMessages ?? [],
  );
  const [status, setStatus] = useState<ChatStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => abortRef.current?.abort();
  }, []);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setStatus("cancelled");
  }, []);

  const reset = useCallback((next?: ChatMessage[]) => {
    abortRef.current?.abort();
    abortRef.current = null;
    setMessages(next ?? []);
    setStatus("idle");
    setError(null);
  }, []);

  const send = useCallback(
    async (raw: ChatSendInput | string) => {
      const input: ChatSendInput =
        typeof raw === "string" ? { text: raw } : raw;
      if (!input.text.trim() && !(input.attachments?.length)) return;

      const userMsg: ChatMessage = {
        id: makeId("u"),
        role: "user",
        content: input.text,
        createdAt: Date.now(),
        status: "done",
        attachments: input.attachments,
        meta: input.meta,
      };
      const assistantId = makeId("a");
      const assistantMsg: ChatMessage = {
        id: assistantId,
        role: "assistant",
        content: "",
        createdAt: Date.now(),
        status: "streaming",
      };

      const history = [...messages, userMsg];
      setMessages([...history, assistantMsg]);
      setStatus("submitted");
      setError(null);

      const ctrl = new AbortController();
      abortRef.current = ctrl;

      try {
        for await (const delta of adapter.send(input, history, ctrl.signal)) {
          if (ctrl.signal.aborted) break;
          if (delta.type === "text" && delta.text) {
            setStatus("streaming");
            setMessages((prev) =>
              prev.map((m) =>
                m.id === assistantId
                  ? { ...m, content: m.content + delta.text }
                  : m,
              ),
            );
          } else if (delta.type === "tool-call" && delta.toolCall) {
            const tc = delta.toolCall;
            setMessages((prev) =>
              prev.map((m) => {
                if (m.id !== assistantId) return m;
                const existing = m.toolCalls ?? [];
                const idx = existing.findIndex((t) => t.id === tc.id);
                const next = [...existing];
                if (idx >= 0) next[idx] = { ...next[idx], ...tc };
                else next.push(tc);
                return { ...m, toolCalls: next };
              }),
            );
          } else if (delta.type === "status" && delta.status) {
            setStatus(delta.status);
          } else if (delta.type === "error") {
            throw new Error(delta.error ?? "stream error");
          } else if (delta.type === "done") {
            break;
          }
        }

        let finished: ChatMessage | undefined;
        setMessages((prev) =>
          prev.map((m) => {
            if (m.id !== assistantId) return m;
            finished = { ...m, status: "done" };
            return finished;
          }),
        );
        setStatus("done");
        if (finished) onFinish?.(finished);
      } catch (err) {
        if (ctrl.signal.aborted) {
          setStatus("cancelled");
          return;
        }
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setStatus("error");
        setMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId ? { ...m, status: "error" } : m,
          ),
        );
        onError?.(err);
      } finally {
        if (abortRef.current === ctrl) abortRef.current = null;
      }
    },
    [adapter, messages, onError, onFinish],
  );

  return useMemo(
    () => ({ messages, status, error, send, cancel, reset, setMessages }),
    [messages, status, error, send, cancel, reset],
  );
}
