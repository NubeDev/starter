// # @nube/starter-ui-chat
//
// Reusable React AI-chat components and hooks. Transport-agnostic:
// supply a `ChatAdapter` (see `./adapters` for echo + SSE helpers, or
// implement your own against starter-ai, MCP, fetch, EventSource…).
//
// Quick start:
//
// ```tsx
// import { Chat, createEchoAdapter } from "@nube/starter-ui-chat";
//
// export function Demo() {
//   const adapter = React.useMemo(() => createEchoAdapter(), []);
//   return <Chat adapter={adapter} title="Demo" />;
// }
// ```
//
// For full control compose the primitives directly:
// `ChatRoot`, `ChatHeader`, `ChatMessageList`, `ChatMessage`,
// `ChatTypingIndicator`, `ChatComposer`, `ChatFooter` with `useChat`.
//
// Styling assumes the consumer has loaded
// `@nube/starter-ui-kit/styles.css`, which provides the design tokens
// (`bg-background`, `bg-primary`, etc.) the components reference.

export * from "./types/index.js";
export * from "./hooks/index.js";
export * from "./adapters/index.js";

export { Chat } from "./components/chat.js";
export type { ChatProps } from "./components/chat.js";
export {
  ChatRoot,
  ChatHeader,
  ChatFooter,
} from "./components/chat-root.js";
export { ChatMessageList } from "./components/chat-message-list.js";
export { ChatMessage } from "./components/chat-message.js";
export { ChatComposer } from "./components/chat-composer.js";
export { ChatEmpty } from "./components/chat-empty.js";
export { ChatTypingIndicator } from "./components/chat-typing-indicator.js";

export { cn, makeId, formatTime } from "./lib/utils.js";
