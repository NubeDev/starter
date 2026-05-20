import * as React from "react";
import type { ChatAdapter, ChatMessage as ChatMessageT } from "../types/index.js";
import { useChat } from "../hooks/use-chat.js";
import { ChatRoot, ChatHeader, ChatFooter } from "./chat-root.js";
import { ChatMessageList } from "./chat-message-list.js";
import { ChatMessage } from "./chat-message.js";
import { ChatComposer } from "./chat-composer.js";
import { ChatEmpty } from "./chat-empty.js";
import { ChatTypingIndicator } from "./chat-typing-indicator.js";

export interface ChatProps {
  adapter: ChatAdapter;
  initialMessages?: ChatMessageT[];
  title?: React.ReactNode;
  headerExtras?: React.ReactNode;
  placeholder?: string;
  emptyTitle?: string;
  emptyDescription?: string;
  suggestions?: Array<{ label: string; value?: string }>;
  className?: string;
  userName?: string;
  assistantName?: string;
  renderMessage?: (m: ChatMessageT) => React.ReactNode;
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
    suggestions,
    className,
    userName,
    assistantName,
    renderMessage,
  } = props;

  const { messages, status, send, cancel } = useChat({
    adapter,
    initialMessages,
  });

  const empty = messages.length === 0;

  return (
    <ChatRoot className={className}>
      {(title || headerExtras) && (
        <ChatHeader>
          {title ? (
            <div className="text-sm font-semibold">{title}</div>
          ) : null}
          <div className="ml-auto">{headerExtras}</div>
        </ChatHeader>
      )}
      <ChatMessageList deps={[messages.length, messages[messages.length - 1]?.content]}>
        {empty ? (
          <ChatEmpty
            title={emptyTitle}
            description={emptyDescription}
            suggestions={suggestions}
            onSuggestion={(v) => send(v)}
          />
        ) : (
          messages.map((m) =>
            renderMessage ? (
              <React.Fragment key={m.id}>{renderMessage(m)}</React.Fragment>
            ) : (
              <ChatMessage
                key={m.id}
                message={m}
                name={m.role === "user" ? userName : assistantName}
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
        />
      </ChatFooter>
    </ChatRoot>
  );
}
