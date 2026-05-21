# @nube/starter-ui-chat

Reusable React AI-chat components and hooks for starter-based apps.

- **Headless transport** — a `ChatAdapter` interface; bring SSE, MCP,
  `starter-ai`, WebSockets, or a mock. Two helpers are included
  (`createEchoAdapter`, `createSseAdapter`).
- **Composable** — drop-in `<Chat />` for the easy path, or compose
  `ChatRoot` + `ChatMessageList` + `ChatMessage` + `ChatComposer` for
  full control.
- **File attachments** — paperclip button, drag-and-drop, paste from
  clipboard. Files round-trip on the message as `ChatAttachment[]` so
  your adapter can upload them however it likes.
- **Persistence** — `persistence={{ key }}` round-trips history through
  `localStorage` (SSR-safe). Swap in `sessionStorage` or any
  `ChatStore` for IndexedDB / server sync.
- **Retry** — `retry()` from `useChat` re-runs the last user turn;
  errored assistant messages show a built-in retry action.
- **Zero I/O in the library** — no fetches, no stores, no globals.
  Same rule as `starter-ui-kit`.
- **Tailwind v4 / shadcn tokens** — assumes the consumer loaded
  `@nube/starter-ui-kit/styles.css`. Components reference the standard
  `bg-background`, `bg-primary`, `bg-muted`, etc.

## Install

```bash
pnpm add @nube/starter-ui-chat
```

Peer deps: `react`, `react-dom`, `@nube/starter-ui-kit`.

## Quick start

```tsx
import { Chat, createEchoAdapter } from "@nube/starter-ui-chat";
import "@nube/starter-ui-kit/styles.css";

export function Demo() {
  const adapter = React.useMemo(() => createEchoAdapter(), []);
  return (
    <Chat
      adapter={adapter}
      title="Assistant"
      persistence={{ key: "demo-chat" }}
      allowAttachments
      showClearButton
    />
  );
}
```

## File attachments

```tsx
<Chat
  adapter={adapter}
  allowAttachments
  acceptAttachments="image/*,.pdf"
  maxAttachments={5}
  maxAttachmentBytes={10 * 1024 * 1024}
/>;
```

Pasting images or drag-dropping files into the composer also works.
Each attachment surfaces on the user message as a `ChatAttachment` —
including the raw `File` blob — so your adapter can `fetch()` with
`multipart/form-data`, base64-encode it, or upload separately and
attach the URL.

## Persistence

```tsx
// localStorage shorthand
<Chat adapter={adapter} persistence={{ key: `agent:${id}` }} />;

// sessionStorage
<Chat
  adapter={adapter}
  persistence={{ key: "demo", storage: sessionStorage }}
/>;

// Custom store (IndexedDB, server, encrypted, …)
const myStore: ChatStore = {
  load: () => /* … */ null,
  save: (msgs) => { /* … */ },
  clear: () => { /* … */ },
};
<Chat adapter={adapter} persistence={myStore} />;
```

The library never touches `window` directly — `createLocalStorageStore`
no-ops on the server, so SSR keeps working.

## Retry

`useChat().retry()` re-runs the last user turn, dropping any trailing
(typically errored or cancelled) assistant message. The default
`<ChatMessage>` shows a retry icon on hover for assistant messages and
prominently for errored ones — both wired into `<Chat>` automatically.

```tsx
const { retry } = useChat({ adapter });
<ChatMessage message={m} onRetry={() => retry()} />;
```

## Wiring a real backend (SSE)

```tsx
import { Chat, createSseAdapter } from "@nube/starter-ui-chat";

const adapter = createSseAdapter({
  url: "/api/chat",
  // server emits `data: {"type":"text","text":"…"}\n\n` and
  // `data: [DONE]\n\n` — defaults handle this. Customise via `parse`.
});

<Chat adapter={adapter} />;
```

## Composing primitives

```tsx
import {
  ChatRoot, ChatHeader, ChatFooter,
  ChatMessageList, ChatMessage,
  ChatComposer, ChatTypingIndicator,
  useChat,
} from "@nube/starter-ui-chat";

function MyChat({ adapter }) {
  const { messages, status, send, cancel } = useChat({ adapter });
  return (
    <ChatRoot>
      <ChatHeader>My Chat</ChatHeader>
      <ChatMessageList deps={[messages.length]}>
        {messages.map((m) => <ChatMessage key={m.id} message={m} />)}
        {status === "submitted" && <ChatTypingIndicator />}
      </ChatMessageList>
      <ChatFooter>
        <ChatComposer status={status} onSend={send} onCancel={cancel} />
      </ChatFooter>
    </ChatRoot>
  );
}
```

## Writing a custom adapter

```ts
import type { ChatAdapter } from "@nube/starter-ui-chat";

export const myAdapter: ChatAdapter = {
  async *send(input, history, signal) {
    const res = await fetch("/api/agents/run", {
      method: "POST",
      signal,
      body: JSON.stringify({ input, history }),
    });
    // …parse and yield ChatStreamDelta values…
    yield { type: "text", text: "hello" };
    yield { type: "done" };
  },
};
```

`ChatStreamDelta` variants: `text` (append tokens), `tool-call`
(upsert into the current assistant message), `status`, `error`, `done`.

## Scope

- ✅ Components, hooks, types, adapters.
- ❌ No transport defaults beyond SSE+echo helpers.
- ❌ No global state, no React context required.
- ❌ No markdown renderer bundled — pass `renderContent` on
  `ChatMessage` (or `renderMessage` on `<Chat />`) to plug in your own.
