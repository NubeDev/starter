import type { ChatMessage, ChatStore } from "../types/index.js";

export interface LocalStorageStoreOptions {
  key: string;
  /** Bump to invalidate older persisted shapes. */
  version?: number;
  /** Defaults to `globalThis.localStorage`. SSR-safe (no-op when absent). */
  storage?: Pick<Storage, "getItem" | "setItem" | "removeItem">;
  /** Cap on persisted messages. Older ones are dropped FIFO. Default: 200. */
  maxMessages?: number;
}

interface Envelope {
  v: number;
  messages: ChatMessage[];
}

// localStorage-backed ChatStore. SSR-safe: if `localStorage` is missing
// (Node, RSC), every method no-ops so persistence silently degrades to
// "in-memory only" instead of throwing.
export function createLocalStorageStore(
  opts: LocalStorageStoreOptions,
): ChatStore {
  const version = opts.version ?? 1;
  const maxMessages = opts.maxMessages ?? 200;
  const storage =
    opts.storage ??
    (typeof globalThis !== "undefined" &&
    (globalThis as { localStorage?: Storage }).localStorage
      ? (globalThis as { localStorage: Storage }).localStorage
      : undefined);

  return {
    load() {
      if (!storage) return null;
      try {
        const raw = storage.getItem(opts.key);
        if (!raw) return null;
        const env = JSON.parse(raw) as Envelope;
        if (!env || env.v !== version || !Array.isArray(env.messages)) {
          return null;
        }
        return env.messages;
      } catch {
        return null;
      }
    },
    save(messages) {
      if (!storage) return;
      try {
        const trimmed =
          messages.length > maxMessages
            ? messages.slice(messages.length - maxMessages)
            : messages;
        const env: Envelope = {
          v: version,
          messages: trimmed.map(stripTransientFields),
        };
        storage.setItem(opts.key, JSON.stringify(env));
      } catch {
        // quota exceeded / serialization failure — drop silently
      }
    },
    clear() {
      if (!storage) return;
      try {
        storage.removeItem(opts.key);
      } catch {
        // ignore
      }
    },
  };
}

// Strip fields that can't (or shouldn't) round-trip through JSON:
// File blobs, object URLs, and any in-flight streaming status.
function stripTransientFields(m: ChatMessage): ChatMessage {
  const status =
    m.status === "streaming" || m.status === "submitted" ? "done" : m.status;
  if (!m.attachments?.length) return { ...m, status };
  return {
    ...m,
    status,
    attachments: m.attachments.map(({ file: _file, url, ...rest }) => {
      // Drop blob: URLs (only valid for the page that minted them).
      const safeUrl = url && url.startsWith("blob:") ? undefined : url;
      return safeUrl ? { ...rest, url: safeUrl } : rest;
    }),
  };
}
