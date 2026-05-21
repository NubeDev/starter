/**
 * Streaming-content primitives: `markdown`, `code`, and the
 * subscription-mode variant of `text`. All three follow the same
 * pattern — server bakes an `initial` value into the IR, an
 * optional `subscribe` subject feeds live chunks via
 * `useStreaming`, and a `mode: "append" | "replace"` decides how
 * each chunk lands.
 *
 * Per SCOPE.md § "Streaming content" (decision S-D5 inherited
 * verbatim from Rubix), the server-emitted `stream_end` sentinel
 * carries `reason: "done" | "error" | "timeout" | "gone"`. The
 * 60-second server-side inactivity timeout produces `"timeout"`;
 * client-side unmount drops the subscription so the channel is
 * GC'd within the same window.
 *
 * `markdown` and `rich_text` delegate prose rendering to the host's
 * markdown / tiptap library — these wrappers are the IR adapters
 * (per SCOPE "Size targets": only the wrapper counts toward the
 * LoC budget).
 */
import { useEffect, useState } from "react";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useStreaming, useStreamingTransport } from "../useStreaming.js";
import type { SubscriptionTransport } from "../useSubscriptions.js";

type StreamMode = "append" | "replace";

// ---------------------------------------------------------------------------
// Markdown — streamable prose. The renderer leaves prose-to-HTML
// conversion to the host (a marked / remark / milkdown adapter); the
// IR wrapper hands the host a plain-string body plus the stream
// state. Hosts that want sanitised HTML wrap the textContent in
// their own renderer; hosts that want raw text get raw text.
// ---------------------------------------------------------------------------
export interface MarkdownNode extends UiComponent {
  type: "markdown";
  content?: string;
  subscribe?: string;
  mode?: StreamMode;
}

export const markdownSpec: ComponentSpec<MarkdownNode> = {
  kind: "markdown",
  Component: ({ node }) => {
    const transport = useStreamingTransport();
    const { value, endedReason } = useStreaming({
      subscribe: node.subscribe,
      initial: node.content ?? "",
      mode: node.mode ?? "append",
      transport,
    });
    return (
      <div className={`prose max-w-none text-sm ${node.style?.className ?? ""}`}>
        <pre className="whitespace-pre-wrap break-words font-sans">{value}</pre>
        {endedReason && endedReason !== "done" ? (
          <p className="mt-1 text-xs text-muted-foreground">
            stream ended: {endedReason}
          </p>
        ) : null}
      </div>
    );
  },
};

// ---------------------------------------------------------------------------
// Code — fenced block, optional `language` hint. Same stream wiring
// as markdown; tokens land in a monospace `<pre>`.
// ---------------------------------------------------------------------------
export interface CodeNode extends UiComponent {
  type: "code";
  content?: string;
  language?: string;
  subscribe?: string;
  mode?: StreamMode;
}

export const codeSpec: ComponentSpec<CodeNode> = {
  kind: "code",
  Component: ({ node }) => {
    const transport = useStreamingTransport();
    const { value, endedReason } = useStreaming({
      subscribe: node.subscribe,
      initial: node.content ?? "",
      mode: node.mode ?? "append",
      transport,
    });
    return (
      <pre
        data-language={node.language}
        className={`overflow-x-auto rounded border bg-muted p-3 text-xs ${node.style?.className ?? ""}`}
      >
        <code>{value}</code>
        {endedReason && endedReason !== "done" ? (
          <span className="mt-2 block text-[10px] text-muted-foreground">
            stream ended: {endedReason}
          </span>
        ) : null}
      </pre>
    );
  },
};

// ---------------------------------------------------------------------------
// StreamingText — `text` variant that opts into the streaming path
// by carrying a `subscribe` subject. The non-streaming `text` spec
// in `Display.tsx` stays as a pure static renderer; this spec is
// registered separately under a different kind id when the IR emits
// `text` nodes with `subscribe`. The renderer dispatches via the
// presence of `subscribe` (handled in the registry barrel).
// ---------------------------------------------------------------------------
export interface StreamingTextNode extends UiComponent {
  type: "text";
  value?: string;
  content?: string;
  subscribe?: string;
  mode?: StreamMode;
  tone?: "default" | "muted" | "danger" | "success" | "warning";
}

export function StreamingTextComponent({ node }: { node: StreamingTextNode }) {
  const transport = useStreamingTransport();
  const initial = node.value ?? node.content ?? "";
  const { value, endedReason } = useStreaming({
    subscribe: node.subscribe,
    initial,
    mode: node.mode ?? "append",
    transport,
  });
  return (
    <p className={`whitespace-pre-wrap text-sm ${node.style?.className ?? ""}`}>
      {value}
      {endedReason && endedReason !== "done" ? (
        <span className="ml-2 text-xs text-muted-foreground">
          ({endedReason})
        </span>
      ) : null}
    </p>
  );
}

// ---------------------------------------------------------------------------
// Timeline — chronological event list, streamable. `mode: "append"`
// (default) adds incoming events to the end; `"replace"` swaps the
// full list. Stream end shows a muted footer marker.
// ---------------------------------------------------------------------------
export interface TimelineEvent {
  id?: string;
  ts?: string | number;
  title?: string;
  body?: string;
  intent?: "info" | "success" | "warning" | "danger";
}
export interface TimelineNode extends UiComponent {
  type: "timeline";
  events?: TimelineEvent[];
  subscribe?: string;
  mode?: StreamMode;
}

function parseEvent(chunk: unknown): TimelineEvent | undefined {
  if (!chunk || typeof chunk !== "object") return undefined;
  return chunk as TimelineEvent;
}

export const timelineSpec: ComponentSpec<TimelineNode> = {
  kind: "timeline",
  Component: ({ node }) => {
    const transport = useStreamingTransport();
    const seed = node.events ?? [];
    const mode: StreamMode = node.mode ?? "append";
    const [events, setEvents] = useState<TimelineEvent[]>(seed);
    const [endedReason, setEndedReason] = useState<string | undefined>();

    // Subscribe directly — the timeline payload is a structured
    // event, not a string, so we don't reuse `useStreaming`.
    useTimelineSubscription(node.subscribe, transport, (chunk) => {
      if (chunk && typeof chunk === "object" && (chunk as { type?: string }).type === "stream_end") {
        setEndedReason((chunk as { reason?: string }).reason);
        return;
      }
      const ev = parseEvent(chunk);
      if (!ev) return;
      setEvents((prev) => (mode === "replace" ? [ev] : [...prev, ev]));
    });

    return (
      <ol className={`flex flex-col gap-3 border-l pl-4 ${node.style?.className ?? ""}`}>
        {events.map((e, i) => (
          <li key={e.id ?? i} className="relative">
            <span className="absolute -left-[1.1rem] top-1 inline-block h-2 w-2 rounded-full bg-primary" />
            <div className="text-xs text-muted-foreground">{String(e.ts ?? "")}</div>
            {e.title ? <div className="text-sm font-medium">{e.title}</div> : null}
            {e.body ? <div className="text-sm text-muted-foreground">{e.body}</div> : null}
          </li>
        ))}
        {endedReason && endedReason !== "done" ? (
          <li className="text-xs text-muted-foreground">stream ended: {endedReason}</li>
        ) : null}
      </ol>
    );
  },
};

function useTimelineSubscription(
  subscribe: string | undefined,
  transport: SubscriptionTransport | undefined,
  onChunk: (chunk: unknown) => void,
) {
  useEffect(() => {
    if (!subscribe || !transport) return;
    return transport.subscribe(
      { key: subscribe, target_node_id: "", slot: subscribe },
      onChunk,
    );
  }, [subscribe, transport, onChunk]);
}
