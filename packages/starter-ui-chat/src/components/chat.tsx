import * as React from "react";
import type {
  ChatAdapter,
  ChatMessage as ChatMessageT,
} from "../types/index.js";
import {
  useChat,
  type UseChatOptions,
  type UseChatPersistence,
} from "../hooks/use-chat.js";
import { ChatRoot, ChatHeader, ChatFooter } from "./chat-root.js";
import { ChatMessageList } from "./chat-message-list.js";
import { ChatMessage } from "./chat-message.js";
import { ChatComposer } from "./chat-composer.js";
import { ChatEmpty, type ChatEmptySuggestion } from "./chat-empty.js";
import { ChatTypingIndicator } from "./chat-typing-indicator.js";

export interface ChatProps {
  adapter: ChatAdapter;
  initialMessages?: ChatMessageT[];
  title?: React.ReactNode;
  headerExtras?: React.ReactNode;
  placeholder?: string;
  emptyTitle?: string;
  emptyDescription?: string;
  emptyIcon?: React.ReactNode;
  suggestions?: Array<ChatEmptySuggestion>;
  className?: string;
  userName?: string;
  assistantName?: string;
  /** Allow file attachments in the composer. */
  allowAttachments?: boolean;
  acceptAttachments?: string;
  maxAttachmentBytes?: number;
  maxAttachments?: number;
  /**
   * Persist messages across reloads — shorthand `{ key: "agent:42" }`
   * or a full `ChatStore`. See `useChat` for details.
   */
  persistence?: UseChatPersistence | UseChatOptions["persistence"];
  /** Show a "Clear" button in the header. Calls `useChat().clear()`. */
  showClearButton?: boolean;
  renderMessage?: (
    m: ChatMessageT,
    helpers: { retry: () => void },
  ) => React.ReactNode;
}

// Opinionated end-to-end chat. For full control compose the pieces
// directly: `ChatRoot` + `ChatMessageList` + `ChatComposer`.
export function Chat(props: ChatProps): React.ReactElement {
  const {
    adapter,
    initialMessages,
    title,
    headerExtras,
    placeholder,
    emptyTitle,
    emptyDescription,
    emptyIcon,
    suggestions,
    className,
    userName,
    assistantName,
    allowAttachments,
    acceptAttachments,
    maxAttachmentBytes,
    maxAttachments,
    persistence,
    showClearButton,
    renderMessage,
  } = props;

  const { messages, status, send, cancel, retry, clear } = useChat({
    adapter,
    initialMessages,
    persistence,
  });

  const empty = messages.length === 0;

  return (
    <ChatRoot className={className}>
      {(title || headerExtras || showClearButton) && (
        <ChatHeader>
          {title ? (
            <div className="text-sm font-semibold">{title}</div>
          ) : null}
          <div className="ml-auto flex items-center gap-2">
            {headerExtras}
            {showClearButton && messages.length > 0 ? (
              <button
                type="button"
                onClick={clear}
                className="rounded-md px-2 py-1 text-xs text-muted-foreground transition hover:bg-muted hover:text-foreground"
              >
                Clear
              </button>
            ) : null}
          </div>
        </ChatHeader>
      )}
      <ChatMessageList
        deps={[messages.length, messages[messages.length - 1]?.content]}
      >
        {empty ? (
          <ChatEmpty
            title={emptyTitle}
            description={emptyDescription}
            icon={emptyIcon}
            suggestions={suggestions}
            onSuggestion={(v) => send(v)}
          />
        ) : (
          messages.map((m) =>
            renderMessage ? (
              <React.Fragment key={m.id}>
                {renderMessage(m, { retry })}
              </React.Fragment>
            ) : (
              <ChatMessage
                key={m.id}
                message={m}
                name={m.role === "user" ? userName : assistantName}
                onRetry={() => retry()}
              />
            ),
          )
        )}
        {status === "submitted" ? (
          <div className="flex">
            <ChatTypingIndicator />
          </div>
        ) : null}
      </ChatMessageList>
      <ChatFooter>
        <ChatComposer
          status={status}
          placeholder={placeholder}
          onSend={send}
          onCancel={cancel}
          allowAttachments={allowAttachments}
          acceptAttachments={acceptAttachments}
          maxAttachmentBytes={maxAttachmentBytes}
          maxAttachments={maxAttachments}
        />
      </ChatFooter>
    </ChatRoot>
  );
}
