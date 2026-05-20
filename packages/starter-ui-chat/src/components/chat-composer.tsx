import * as React from "react";
import { cn } from "../lib/utils.js";
import { useAutosizeTextarea } from "../hooks/use-autosize-textarea.js";
import type { ChatSendInput, ChatStatus } from "../types/index.js";

export interface ChatComposerProps {
  status?: ChatStatus;
  placeholder?: string;
  disabled?: boolean;
  maxRows?: number;
  onSend: (input: ChatSendInput) => void;
  onCancel?: () => void;
  className?: string;
  children?: React.ReactNode;
  // Override the default submit button. Receives current state.
  renderSubmit?: (state: {
    status: ChatStatus;
    canSubmit: boolean;
    canCancel: boolean;
  }) => React.ReactNode;
}

export const ChatComposer = React.forwardRef<
  HTMLFormElement,
  ChatComposerProps
>((props, ref) => {
  const {
    status = "idle",
    placeholder = "Send a message…",
    disabled,
    maxRows = 8,
    onSend,
    onCancel,
    className,
    children,
    renderSubmit,
  } = props;
  const [value, setValue] = React.useState("");
  const taRef = useAutosizeTextarea(value, maxRows);

  const isStreaming = status === "streaming" || status === "submitted";
  const canSubmit = !disabled && !isStreaming && value.trim().length > 0;
  const canCancel = isStreaming && !!onCancel;

  const submit = React.useCallback(() => {
    if (!canSubmit) return;
    onSend({ text: value.trim() });
    setValue("");
  }, [canSubmit, onSend, value]);

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <form
      ref={ref}
      data-slot="chat-composer"
      className={cn(
        "flex w-full items-end gap-2 rounded-2xl border bg-background p-2 shadow-sm focus-within:ring-2 focus-within:ring-ring/30",
        className,
      )}
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <textarea
        ref={taRef}
        rows={1}
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={onKeyDown}
        className={cn(
          "min-h-[2.25rem] flex-1 resize-none bg-transparent px-2 py-1.5 text-sm outline-none placeholder:text-muted-foreground",
        )}
      />
      {children}
      {renderSubmit ? (
        renderSubmit({ status, canSubmit, canCancel })
      ) : (
        <DefaultSubmit
          status={status}
          canSubmit={canSubmit}
          canCancel={canCancel}
          onCancel={onCancel}
        />
      )}
    </form>
  );
});
ChatComposer.displayName = "ChatComposer";

function DefaultSubmit({
  status,
  canSubmit,
  canCancel,
  onCancel,
}: {
  status: ChatStatus;
  canSubmit: boolean;
  canCancel: boolean;
  onCancel?: () => void;
}) {
  if (canCancel) {
    return (
      <button
        type="button"
        onClick={onCancel}
        aria-label="Cancel"
        className="inline-flex h-9 w-9 items-center justify-center rounded-full bg-destructive text-destructive-foreground hover:opacity-90"
      >
        <span className="block h-2.5 w-2.5 bg-current" />
      </button>
    );
  }
  return (
    <button
      type="submit"
      disabled={!canSubmit}
      aria-label="Send"
      data-status={status}
      className={cn(
        "inline-flex h-9 w-9 items-center justify-center rounded-full bg-primary text-primary-foreground transition",
        "disabled:cursor-not-allowed disabled:opacity-50 hover:enabled:opacity-90",
      )}
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="h-4 w-4"
        aria-hidden
      >
        <path d="M5 12l14-7-7 14-2-5-5-2z" />
      </svg>
    </button>
  );
}
