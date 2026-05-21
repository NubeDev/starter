import * as React from "react";
import { cn } from "../lib/utils.js";
import { useAutosizeTextarea } from "../hooks/use-autosize-textarea.js";
import {
  fileToAttachment,
  formatBytes,
  isImageAttachment,
} from "../lib/attachments.js";
import type {
  ChatAttachment,
  ChatSendInput,
  ChatStatus,
} from "../types/index.js";

export interface ChatComposerProps {
  status?: ChatStatus;
  placeholder?: string;
  disabled?: boolean;
  maxRows?: number;
  onSend: (input: ChatSendInput) => void;
  onCancel?: () => void;
  className?: string;
  children?: React.ReactNode;
  /** Enable the paperclip / drag-drop affordance. */
  allowAttachments?: boolean;
  /** `accept` value for the file picker (e.g. "image/*,.pdf"). */
  acceptAttachments?: string;
  /** Cap on combined file size (bytes); rejects oversized picks. */
  maxAttachmentBytes?: number;
  /** Cap on number of attachments per message. */
  maxAttachments?: number;
  /** Override the default submit button. Receives current state. */
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
    allowAttachments = false,
    acceptAttachments,
    maxAttachmentBytes,
    maxAttachments = 10,
    renderSubmit,
  } = props;
  const [value, setValue] = React.useState("");
  const [attachments, setAttachments] = React.useState<ChatAttachment[]>([]);
  const [dragOver, setDragOver] = React.useState(false);
  const taRef = useAutosizeTextarea(value, maxRows);
  const fileInputRef = React.useRef<HTMLInputElement | null>(null);

  const isStreaming = status === "streaming" || status === "submitted";
  const canSubmit =
    !disabled &&
    !isStreaming &&
    (value.trim().length > 0 || attachments.length > 0);
  const canCancel = isStreaming && !!onCancel;

  // Revoke object URLs we no longer reference when the composer unmounts.
  React.useEffect(() => {
    return () => {
      for (const a of attachments) {
        if (a.url?.startsWith("blob:")) URL.revokeObjectURL(a.url);
      }
    };
    // intentionally empty — we want this only on unmount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const addFiles = React.useCallback(
    (files: FileList | File[] | null) => {
      if (!files) return;
      const list = Array.from(files);
      if (!list.length) return;
      setAttachments((prev) => {
        const room = Math.max(0, maxAttachments - prev.length);
        if (room === 0) return prev;
        const picked = list.slice(0, room).filter((f) => {
          if (maxAttachmentBytes && f.size > maxAttachmentBytes) return false;
          return true;
        });
        if (!picked.length) return prev;
        return [...prev, ...picked.map(fileToAttachment)];
      });
    },
    [maxAttachmentBytes, maxAttachments],
  );

  const removeAttachment = React.useCallback((id: string) => {
    setAttachments((prev) => {
      const out: ChatAttachment[] = [];
      for (const a of prev) {
        if (a.id === id) {
          if (a.url?.startsWith("blob:")) URL.revokeObjectURL(a.url);
        } else {
          out.push(a);
        }
      }
      return out;
    });
  }, []);

  const submit = React.useCallback(() => {
    if (!canSubmit) return;
    onSend({
      text: value.trim(),
      attachments: attachments.length ? attachments : undefined,
    });
    setValue("");
    setAttachments([]);
  }, [canSubmit, onSend, value, attachments]);

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      submit();
    }
  };

  const onPaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    if (!allowAttachments) return;
    const files: File[] = [];
    for (const item of Array.from(e.clipboardData.items)) {
      if (item.kind === "file") {
        const f = item.getAsFile();
        if (f) files.push(f);
      }
    }
    if (files.length) {
      e.preventDefault();
      addFiles(files);
    }
  };

  const onDrop = (e: React.DragEvent<HTMLFormElement>) => {
    if (!allowAttachments) return;
    e.preventDefault();
    setDragOver(false);
    addFiles(e.dataTransfer.files);
  };

  const onDragOver = (e: React.DragEvent<HTMLFormElement>) => {
    if (!allowAttachments) return;
    if (!Array.from(e.dataTransfer.types).includes("Files")) return;
    e.preventDefault();
    setDragOver(true);
  };

  return (
    <form
      ref={ref}
      data-slot="chat-composer"
      data-drag-over={dragOver ? "" : undefined}
      className={cn(
        "group/composer relative flex w-full flex-col gap-2 rounded-2xl border bg-background p-2 shadow-sm transition focus-within:border-ring/40 focus-within:ring-2 focus-within:ring-ring/20",
        dragOver && "border-primary/60 ring-2 ring-primary/30",
        className,
      )}
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
      onDrop={onDrop}
      onDragOver={onDragOver}
      onDragLeave={() => setDragOver(false)}
    >
      {attachments.length > 0 && (
        <ul
          data-slot="chat-composer-attachments"
          className="flex flex-wrap gap-2 px-1 pt-1"
        >
          {attachments.map((a) => (
            <li
              key={a.id}
              className="group/att relative flex items-center gap-2 rounded-lg border bg-muted/50 py-1 pl-1 pr-7 text-xs"
            >
              {isImageAttachment(a) && a.url ? (
                <img
                  src={a.url}
                  alt=""
                  className="h-8 w-8 rounded-md object-cover"
                />
              ) : (
                <span className="flex h-8 w-8 items-center justify-center rounded-md bg-background text-[10px] font-semibold uppercase text-muted-foreground">
                  {(a.name.split(".").pop() || "file").slice(0, 4)}
                </span>
              )}
              <span className="flex flex-col">
                <span className="max-w-[10rem] truncate font-medium">
                  {a.name}
                </span>
                {a.sizeBytes ? (
                  <span className="text-[10px] text-muted-foreground">
                    {formatBytes(a.sizeBytes)}
                  </span>
                ) : null}
              </span>
              <button
                type="button"
                aria-label={`Remove ${a.name}`}
                onClick={() => removeAttachment(a.id)}
                className="absolute right-1 top-1 inline-flex h-5 w-5 items-center justify-center rounded-full bg-background/80 text-muted-foreground opacity-80 hover:bg-background hover:text-foreground"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="h-3 w-3"
                  aria-hidden
                >
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex w-full items-end gap-1">
        {allowAttachments && (
          <>
            <input
              ref={fileInputRef}
              type="file"
              multiple
              hidden
              accept={acceptAttachments}
              onChange={(e) => {
                addFiles(e.target.files);
                e.target.value = "";
              }}
            />
            <button
              type="button"
              aria-label="Attach files"
              disabled={disabled || attachments.length >= maxAttachments}
              onClick={() => fileInputRef.current?.click()}
              className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full text-muted-foreground transition hover:bg-muted hover:text-foreground disabled:opacity-40"
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
                <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48" />
              </svg>
            </button>
          </>
        )}
        <textarea
          ref={taRef}
          rows={1}
          value={value}
          disabled={disabled}
          placeholder={placeholder}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
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
      </div>
      {dragOver && (
        <div
          className="pointer-events-none absolute inset-0 flex items-center justify-center rounded-2xl bg-primary/5 text-xs font-medium text-primary"
          aria-hidden
        >
          Drop files to attach
        </div>
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
        className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-destructive text-destructive-foreground transition hover:opacity-90"
      >
        <span className="block h-2.5 w-2.5 rounded-[2px] bg-current" />
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
        "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground transition",
        "disabled:cursor-not-allowed disabled:opacity-40 hover:enabled:opacity-90",
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
