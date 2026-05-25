// `/chat` — streaming chat surface for the rubix dashboard
// assistant. Posts each turn to `POST /api/v1/chat/stream` (the SSE
// route shipped in `rubix/crates/rubix-agent/src/routes/chat_stream.rs`)
// and renders the runner events as they arrive:
//
//   - `text`     → appended to the in-progress assistant bubble.
//   - `tool_use` → rendered inline as a pill (`⚙ name`) so the
//                  operator can see what the model dispatched.
//   - `done`     → seals the bubble; the cursor disappears.
//   - `error`    → spawns a red error bubble.
//
// Backend wire shape is documented at the top of `chat_stream.rs`.
// We deliberately use `fetch` + a `ReadableStream` reader instead of
// `EventSource` because the route is a POST (so the browser's
// EventSource implementation cannot be used) and we want to send a
// JSON body. The cookie-based auth that protects every other rubix
// route works unchanged because `client.starter.fetch` is the same
// fetch every other hook uses.

import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useRef, useState } from 'react'
import { useRubixClient } from '@nube/rubix-client-react'
import { PromptInputBox } from '@/components/ui/ai-prompt-box'
import { cn } from '@/lib/utils'

const STREAM_PATH = '/api/v1/chat/stream'

/** One inline tool dispatch recorded as the model emitted it. */
type ToolPill = { id: string; name: string }

/** A single chat bubble. `streaming=true` shows the typing cursor. */
type Turn =
  | { role: 'user'; id: string; text: string }
  | {
      role: 'assistant'
      id: string
      text: string
      tools: ToolPill[]
      streaming: boolean
    }
  | { role: 'error'; id: string; text: string }

/** Wire frames produced by `routes/chat_stream.rs::ChatFrame`. */
type ChatFrame =
  | { type: 'connected'; model?: string }
  | { type: 'text'; delta: string }
  | { type: 'tool_use'; id?: string; name: string; input?: unknown }
  | {
      type: 'done'
      input_tokens: number
      output_tokens: number
      cost_usd: number
      duration_ms: number
    }
  | { type: 'error'; message: string }

function Bubble({ turn }: { turn: Turn }) {
  const mine = turn.role === 'user'
  return (
    <div className={cn('flex w-full', mine ? 'justify-end' : 'justify-start')}>
      <div
        className={cn(
          'max-w-[85%] whitespace-pre-wrap rounded-2xl px-4 py-3 text-sm shadow-sm',
          mine && 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-text)]',
          turn.role === 'assistant' &&
            'bg-[color:var(--color-card)] text-[color:var(--color-text)]',
          turn.role === 'error' && 'bg-red-500/10 text-red-600 dark:text-red-300',
        )}
      >
        {turn.role === 'assistant' && turn.tools.length > 0 ? (
          <div className="mb-2 flex flex-wrap gap-1.5">
            {turn.tools.map((t) => (
              <span
                key={t.id}
                className="rounded-full bg-[color:var(--color-leaf)]/15 px-2 py-0.5 text-[11px] font-mono text-[color:var(--color-leaf)]"
                title={t.name}
              >
                ⚙ {t.name}
              </span>
            ))}
          </div>
        ) : null}
        {turn.text}
        {turn.role === 'assistant' && turn.streaming ? (
          <span className="ml-0.5 inline-block w-1.5 animate-pulse">▌</span>
        ) : null}
      </div>
    </div>
  )
}

/**
 * Pull `data: {json}\n\n` frames out of a Server-Sent-Events text
 * buffer. Returns the parsed JSON payloads and the unconsumed tail
 * that the caller should prepend to the next chunk. Tolerates lone
 * `:` heartbeat lines and unknown event names — only `data:` lines
 * are parsed.
 */
function parseSseChunk(buf: string): { frames: ChatFrame[]; rest: string } {
  const frames: ChatFrame[] = []
  let rest = buf
  for (;;) {
    const sep = rest.indexOf('\n\n')
    if (sep === -1) break
    const block = rest.slice(0, sep)
    rest = rest.slice(sep + 2)
    const dataLines = block
      .split('\n')
      .filter((l) => l.startsWith('data:'))
      .map((l) => l.slice(5).trimStart())
    if (dataLines.length === 0) continue
    try {
      frames.push(JSON.parse(dataLines.join('\n')) as ChatFrame)
    } catch {
      // Drop malformed payloads silently — the backend is the only
      // producer so a parse error here is a developer bug, not
      // something to surface in chat UX.
    }
  }
  return { frames, rest }
}

function ChatPage() {
  const client = useRubixClient()
  const [turns, setTurns] = useState<Turn[]>([])
  const [busy, setBusy] = useState(false)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const abortRef = useRef<AbortController | null>(null)

  // Auto-scroll on every turn / streaming-delta update so the
  // operator does not have to chase the cursor.
  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: 'smooth',
    })
  }, [turns])

  // Cancel any in-flight stream when the route unmounts.
  useEffect(() => () => abortRef.current?.abort(), [])

  const send = async (message: string) => {
    const trimmed = message.trim()
    if (!trimmed || busy) return

    // 1. Push the user bubble + an empty streaming assistant bubble
    //    so the cursor appears immediately and the next `text`
    //    frame appends in place.
    const userId = crypto.randomUUID()
    const assistantId = crypto.randomUUID()
    setTurns((t) => [
      ...t,
      { role: 'user', id: userId, text: trimmed },
      {
        role: 'assistant',
        id: assistantId,
        text: '',
        tools: [],
        streaming: true,
      },
    ])
    setBusy(true)

    // 2. Cancel any prior in-flight stream before opening a new one.
    abortRef.current?.abort()
    const ctrl = new AbortController()
    abortRef.current = ctrl

    // Helper that mutates only the in-flight assistant bubble.
    const patch = (fn: (turn: Extract<Turn, { role: 'assistant' }>) => Turn) =>
      setTurns((t) =>
        t.map((x) => (x.id === assistantId && x.role === 'assistant' ? fn(x) : x)),
      )

    try {
      const res = await client.starter.fetch(
        `${client.starter.baseUrl}${STREAM_PATH}`,
        {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            accept: 'text/event-stream',
            'accept-language':
              typeof navigator !== 'undefined' ? navigator.language : 'en',
          },
          body: JSON.stringify({ prompt: trimmed }),
          signal: ctrl.signal,
        },
      )
      if (!res.ok || !res.body) {
        const detail = (await res.text().catch(() => '')) || `HTTP ${res.status}`
        throw new Error(detail.slice(0, 200))
      }

      const reader = res.body.getReader()
      const decoder = new TextDecoder('utf-8')
      let buf = ''
      // eslint-disable-next-line no-constant-condition
      while (true) {
        const { value, done } = await reader.read()
        if (done) break
        buf += decoder.decode(value, { stream: true })
        const { frames, rest } = parseSseChunk(buf)
        buf = rest
        for (const frame of frames) {
          if (frame.type === 'text') {
            patch((a) => ({ ...a, text: a.text + frame.delta }))
          } else if (frame.type === 'tool_use') {
            patch((a) => ({
              ...a,
              tools: [
                ...a.tools,
                { id: frame.id ?? crypto.randomUUID(), name: frame.name },
              ],
            }))
          } else if (frame.type === 'error') {
            // Convert in-band error into an error bubble; mark the
            // in-progress assistant bubble as no longer streaming
            // so its cursor disappears.
            patch((a) => ({ ...a, streaming: false }))
            setTurns((t) => [
              ...t,
              { role: 'error', id: crypto.randomUUID(), text: frame.message },
            ])
          } else if (frame.type === 'done') {
            patch((a) => ({ ...a, streaming: false }))
          }
          // `connected` is ignored — UI does not need to surface it.
        }
      }
      // Stream closed without an explicit `done` (e.g. server
      // dropped the connection). Stop the cursor in any case so the
      // UI does not look stuck.
      patch((a) => ({ ...a, streaming: false }))
    } catch (err: unknown) {
      if ((err as { name?: string })?.name === 'AbortError') {
        patch((a) => ({ ...a, streaming: false }))
      } else {
        patch((a) => ({ ...a, streaming: false }))
        setTurns((t) => [
          ...t,
          {
            role: 'error',
            id: crypto.randomUUID(),
            text: err instanceof Error ? err.message : String(err),
          },
        ])
      }
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="mx-auto flex h-[calc(100svh-7rem)] w-full max-w-3xl flex-col gap-4 px-4 py-6">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold">Dashboard assistant</h1>
        <p className="text-sm text-[color:var(--color-text-muted)]">
          Ask me to build a dashboard, summarise system state, or list what you
          already have. Streamed via <code>POST {STREAM_PATH}</code>.
        </p>
      </header>

      <div
        ref={scrollRef}
        className="flex-1 space-y-3 overflow-y-auto rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-bg-soft)] p-4"
      >
        {turns.length === 0 ? (
          <div className="grid h-full place-items-center text-sm opacity-60">
            Try: <code className="ml-1">make me an iot dashboard</code>
          </div>
        ) : (
          turns.map((t) => <Bubble key={t.id} turn={t} />)
        )}
      </div>

      <PromptInputBox
        onSend={(message) => void send(message)}
        isLoading={busy}
        placeholder="Message the dashboard assistant…"
      />
    </div>
  )
}

export const Route = createFileRoute('/chat')({ component: ChatPage })
