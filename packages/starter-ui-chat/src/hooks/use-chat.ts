import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ChatAdapter,
  ChatMessage,
  ChatSendInput,
  ChatStatus,
  ChatStore,
} from "../types/index.js";
import { makeId } from "../lib/utils.js";
import {
  createLocalStorageStore,
  type LocalStorageStoreOptions,
} from "../lib/store.js";

export interface UseChatPersistence {
  /** localStorage key. Required when using the shorthand. */
  key: string;
  /** Override storage (sessionStorage, in-memory, …). */
  storage?: LocalStorageStoreOptions["storage"];
  /** Bump to invalidate older shapes after a breaking change. */
  version?: number;
  maxMessages?: number;
}

export interface UseChatOptions {
  adapter: ChatAdapter;
  initialMessages?: ChatMessage[];
  onError?: (err: unknown) => void;
  onFinish?: (assistant: ChatMessage) => void;
  /**
   * Persist messages across reloads. Pass either a shorthand
   * `{ key: "agent:42" }` (localStorage by default) or a custom
   * `ChatStore` implementation.
   *
   * If both `persistence` and `initialMessages` are provided, the
   * persisted snapshot wins; `initialMessages` is used only when the
   * store is empty.
   */
  persistence?: UseChatPersistence | ChatStore;
}

export interface UseChatReturn {
  messages: ChatMessage[];
  status: ChatStatus;
  error: string | null;
  send: (input: ChatSendInput | string) => Promise<void>;
  cancel: () => void;
  /**
   * Re-run the last user turn. Drops any trailing assistant message
   * (typically the errored or cancelled one) and replays the user
   * input through the adapter. No-op if there's no user message.
   */
  retry: () => Promise<void>;
  reset: (messages?: ChatMessage[]) => void;
  setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
  /** Wipe persisted history (if any) and in-memory messages. */
  clear: () => void;
}

function resolveStore(p: UseChatOptions["persistence"]): ChatStore | null {
  if (!p) return null;
  if (typeof (p as ChatStore).load === "function") return p as ChatStore;
  return createLocalStorageStore(p as UseChatPersistence);
}

// Headless chat state machine. The view layer just renders `messages`
// and calls `send`/`cancel`/`retry`. Transport is the adapter's
// concern; persistence is the store's concern.
export function useChat(opts: UseChatOptions): UseChatReturn {
  const { adapter, initialMessages, onError, onFinish } = opts;
  // Store is created once; re-creating it would re-hydrate mid-session.
  const storeRef = useRef<ChatStore | null>(null);
  if (storeRef.current === null) {
    storeRef.current = resolveStore(opts.persistence);
  }
  const store = storeRef.current;

  const [messages, setMessages] = useState<ChatMessage[]>(() => {
    const persisted = store?.load();
    if (persisted && persisted.length) return persisted;
    return initialMessages ?? [];
  });
  const [status, setStatus] = useState<ChatStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  // Always-current snapshot for callbacks that close over messages.
  const messagesRef = useRef(messages);
  useEffect(() => {
    messagesRef.current = messages;
    // Skip persisting mid-stream to avoid quota churn; the post-stream
    // state transition will save the final version.
    if (store && status !== "streaming") store.save(messages);
  }, [messages, status, store]);

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

  const clear = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setMessages([]);
    setStatus("idle");
    setError(null);
    store?.clear();
  }, [store]);

  const runTurn = useCallback(
    async (input: ChatSendInput, baseHistory: ChatMessage[]) => {
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

      const history = [...baseHistory, userMsg];
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
    [adapter, onError, onFinish],
  );

  const send = useCallback(
    async (raw: ChatSendInput | string) => {
      const input: ChatSendInput =
        typeof raw === "string" ? { text: raw } : raw;
      if (!input.text.trim() && !input.attachments?.length) return;
      await runTurn(input, messagesRef.current);
    },
    [runTurn],
  );

  const retry = useCallback(async () => {
    const all = messagesRef.current;
    // Find the last user message; drop everything after it.
    let lastUserIdx = -1;
    for (let i = all.length - 1; i >= 0; i--) {
      if (all[i]?.role === "user") {
        lastUserIdx = i;
        break;
      }
    }
    if (lastUserIdx < 0) return;
    const user = all[lastUserIdx];
    if (!user) return;
    const base = all.slice(0, lastUserIdx);
    setMessages(base);
    await runTurn(
      {
        text: user.content,
        attachments: user.attachments,
        meta: user.meta,
      },
      base,
    );
  }, [runTurn]);

  return useMemo(
    () => ({
      messages,
      status,
      error,
      send,
      cancel,
      retry,
      reset,
      setMessages,
      clear,
    }),
    [messages, status, error, send, cancel, retry, reset, clear],
  );
}
