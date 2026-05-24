// React context that carries the resolved `ChatMessages` from
// `<Chat>` down to the composer, empty state, typing indicator, and
// message bubbles. Components consume via `useChatMessages()`;
// callers can still pass explicit per-prop overrides where the API
// already accepted them (e.g. `<ChatEmpty title="…">`).

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  DEFAULT_CHAT_MESSAGES,
  mergeChatMessages,
  type ChatMessages,
} from "./messages.js";

const ChatI18nContext = createContext<ChatMessages>(DEFAULT_CHAT_MESSAGES);

export interface ChatI18nProviderProps {
  /** Partial override merged on top of `DEFAULT_CHAT_MESSAGES`. */
  value?: Partial<ChatMessages>;
  children: ReactNode;
}

/** Provider — `<Chat>` wraps its tree in this automatically when
 * given an `i18n` prop. Composing the primitives by hand? Wrap
 * them yourself. */
export function ChatI18nProvider({ value, children }: ChatI18nProviderProps) {
  const merged = useMemo(() => mergeChatMessages(value), [value]);
  return (
    <ChatI18nContext.Provider value={merged}>
      {children}
    </ChatI18nContext.Provider>
  );
}

/** Read the current `ChatMessages`. Falls back to English defaults
 * outside any provider. */
export function useChatMessages(): ChatMessages {
  return useContext(ChatI18nContext);
}
