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
import {
  ChatI18nProvider,
  useChatMessages,
  type ChatMessages,
} from "../i18n/index.js";

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
  /** Partial override of the package's user-visible strings (composer
   * placeholder, action labels, typing-indicator a11y, …). Hosts
   * derive this from their own translation hook. */
  i18n?: Partial<ChatMessages>;
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
    i18n,
    renderMessage,
  } = props;

  const { messages, status, send, cancel, retry, clear } = useChat({
    adapter,
    initialMessages,
    persistence,
  });

  const empty = messages.length === 0;

  return (
    <ChatI18nProvider value={i18n}>
      <ChatBody
        className={className}
        title={title}
        headerExtras={headerExtras}
        showClearButton={showClearButton}
        messages={messages}
        status={status}
        send={send}
        cancel={cancel}
        retry={retry}
        clear={clear}
        empty={empty}
        emptyTitle={emptyTitle}
        emptyDescription={emptyDescription}
        emptyIcon={emptyIcon}
        suggestions={suggestions}
        userName={userName}
        assistantName={assistantName}
        placeholder={placeholder}
        allowAttachments={allowAttachments}
        acceptAttachments={acceptAttachments}
        maxAttachmentBytes={maxAttachmentBytes}
        maxAttachments={maxAttachments}
        renderMessage={renderMessage}
      />
    </ChatI18nProvider>
  );
}

interface ChatBodyProps {
  className?: string;
  title?: React.ReactNode;
  headerExtras?: React.ReactNode;
  showClearButton?: boolean;
  messages: ChatMessageT[];
  status: ReturnType<typeof useChat>["status"];
  send: ReturnType<typeof useChat>["send"];
  cancel: ReturnType<typeof useChat>["cancel"];
  retry: ReturnType<typeof useChat>["retry"];
  clear: ReturnType<typeof useChat>["clear"];
  empty: boolean;
  emptyTitle?: string;
  emptyDescription?: string;
  emptyIcon?: React.ReactNode;
  suggestions?: Array<ChatEmptySuggestion>;
  userName?: string;
  assistantName?: string;
  placeholder?: string;
  allowAttachments?: boolean;
  acceptAttachments?: string;
  maxAttachmentBytes?: number;
  maxAttachments?: number;
  renderMessage?: (
    m: ChatMessageT,
    helpers: { retry: () => void },
  ) => React.ReactNode;
}

function ChatBody(props: ChatBodyProps): React.ReactElement {
  const {
    className,
    title,
    headerExtras,
    showClearButton,
    messages,
    status,
    send,
    cancel,
    retry,
    clear,
    empty,
    emptyTitle,
    emptyDescription,
    emptyIcon,
    suggestions,
    userName,
    assistantName,
    placeholder,
    allowAttachments,
    acceptAttachments,
    maxAttachmentBytes,
    maxAttachments,
    renderMessage,
  } = props;
  const chatMessages = useChatMessages();

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
                {chatMessages.clear}
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
