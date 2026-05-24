// Localizable strings the package emits at runtime.
//
// The package stays react-intl-free (matches the rest of the kit —
// see `starter-ui-flow/src/i18n/messages.ts` and `starter-ui-kit`'s
// `ConfigDrawer`). Hosts derive a `ChatMessages` object from their
// own translation hook and pass it via `Chat.i18n`, or wrap the
// primitives in `<ChatI18nProvider>` directly.
//
// Every visible string the package owns lives here. Bubble names
// (`userName` / `assistantName`) and the empty-state `description`
// stay as explicit props on `<Chat>` because they're host content,
// not UI chrome.

export interface ChatMessages {
  /** `<ChatEmpty>` headline default. */
  emptyTitle: string;
  /** `<ChatTypingIndicator>` `aria-label` default. */
  typing: string;
  /** `<ChatComposer>` textarea placeholder default. */
  composerPlaceholder: string;
  /** Drag-over banner inside the composer when files are dragged in. */
  dropFilesToAttach: string;
  /** Attach-files button `aria-label`. */
  attachFiles: string;
  /** Cancel button `aria-label` (shown while streaming). */
  cancel: string;
  /** Send button `aria-label`. */
  send: string;
  /** Copy-message action label (also `aria-label` + `title`). */
  copy: string;
  /** Confirmation after copy succeeds. */
  copied: string;
  /** Retry action label on errored assistant messages. */
  retry: string;
  /** Clear-conversation button label in the default `<Chat>` header. */
  clear: string;
}

/** Default English messages. */
export const DEFAULT_CHAT_MESSAGES: ChatMessages = {
  emptyTitle: "How can I help?",
  typing: "Assistant is typing",
  composerPlaceholder: "Send a message…",
  dropFilesToAttach: "Drop files to attach",
  attachFiles: "Attach files",
  cancel: "Cancel",
  send: "Send",
  copy: "Copy",
  copied: "Copied",
  retry: "Retry",
  clear: "Clear",
};

/** Merge a partial override on top of `DEFAULT_CHAT_MESSAGES`. */
export function mergeChatMessages(
  override: Partial<ChatMessages> | undefined,
): ChatMessages {
  if (!override) return DEFAULT_CHAT_MESSAGES;
  return { ...DEFAULT_CHAT_MESSAGES, ...override };
}
