// `/admin/console` — admin introspection + test console.
//
// Browses every registry projected by `GET /api/v1/admin/registry`
// (tools, nodes, rules, templates, tables, skills, extensions),
// renders the detail view for the selected item (id, source,
// summary, metadata, input schema), and — for tools — lets an
// operator fire a synchronous *or* SSE-streaming invoke against
// `POST /api/v1/admin/registry/tools/{id}/invoke[/stream]`.
//
// Backend surfaces consumed by this page:
//   - `GET  /api/v1/admin/registry?kinds=<kind>` → per-kind page
//   - `GET  /api/v1/admin/registry/{kind}s/{id}` → item detail
//   - `POST /api/v1/admin/registry/tools/{id}/invoke`        (sync)
//   - `POST /api/v1/admin/registry/tools/{id}/invoke/stream` (SSE)
//
// Everything goes through `client.starter.fetch(...)` so CSRF and
// auth cookies ride along automatically.

import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useMemo, useState, type FormEvent } from 'react'
import { useIntl } from 'react-intl'
import { useQuery } from '@tanstack/react-query'
import {
  Boxes,
  Cpu,
  Database,
  Layers,
  Play,
  Radio,
  ShieldCheck,
  Sparkles,
  Wand2,
  type LucideIcon,
} from 'lucide-react'
import { Button, Skeleton } from '@nube/starter-ui-kit'
import { useRubixClient } from '@nube/rubix-client-react'
import { readCsrfHeader } from '@nube/starter-client-ts'
import Form from '@rjsf/core'
import validator from '@rjsf/validator-ajv8'
import type { RJSFSchema } from '@rjsf/utils'
import { ErrorBoundary } from '@/components/error-boundary'

// ---------- types mirroring rubix-spi::dto::admin -----------------

type RegistryKind =
  | 'tool'
  | 'node'
  | 'rule'
  | 'template'
  | 'table'
  | 'skill'
  | 'extension'

interface ItemSource {
  kind: 'builtin' | 'extension' | 'starter'
  id?: string
}

interface RegistryItem {
  id: string
  label: string
  summary: string
  source: ItemSource
  input_schema?: unknown
  output_schema?: unknown
  metadata?: Record<string, unknown>
}

interface Page<T> {
  items: T[]
  next_cursor?: string | null
  total?: number | null
}

const KIND_META: Record<
  RegistryKind,
  { icon: LucideIcon; pathSegment: string; titleKey: string; title: string; descKey: string; desc: string }
> = {
  tool: {
    icon: Wand2,
    pathSegment: 'tools',
    titleKey: 'console.kind.tool.title',
    title: 'Tools',
    descKey: 'console.kind.tool.desc',
    desc:
      'Callable units the agent (and MCP clients) can dispatch. Each tool declares an input JSON Schema, runs synchronously, and returns structured output. Backed by the in-process tool registry plus anything contributed by extensions. This is the only kind you can fire from this console.',
  },
  node: {
    icon: Boxes,
    pathSegment: 'nodes',
    titleKey: 'console.kind.node.title',
    title: 'Flow nodes',
    descKey: 'console.kind.node.desc',
    desc:
      'Node kinds the flow engine knows how to execute. A flow is a graph of these nodes wired by ports; the engine resolves each kind to its handler at run-time. Use this view to confirm a node kind is registered and inspect the port/config schema before referencing it from a flow.',
  },
  rule: {
    icon: ShieldCheck,
    pathSegment: 'rules',
    titleKey: 'console.kind.rule.title',
    title: 'Cleaner rules',
    descKey: 'console.kind.rule.desc',
    desc:
      'Anomaly / quality rules the cleaner pipeline evaluates against incoming warehouse data. Each rule has an id, a target table, and the predicate metadata that decides which rows are flagged. Browse here to audit which rules are loaded; enable/disable lives in /admin/warehouse.',
  },
  template: {
    icon: Sparkles,
    pathSegment: 'templates',
    titleKey: 'console.kind.template.title',
    title: 'Warehouse templates',
    descKey: 'console.kind.template.desc',
    desc:
      'Named read-shape templates over the warehouse — typically parameterised SQL or mart definitions. Tools and dashboards reference templates by name to ask "give me this projection of these tables" without embedding SQL. Metadata shows the target tables and an SQL preview.',
  },
  table: {
    icon: Database,
    pathSegment: 'tables',
    titleKey: 'console.kind.table.title',
    title: 'Warehouse tables',
    descKey: 'console.kind.table.desc',
    desc:
      'Tables the warehouse currently exposes — contributed by builtin marts and by extensions that declare schemas. This is the catalog the templates above project against; for the full row browser see /admin/warehouse-explorer.',
  },
  skill: {
    icon: Cpu,
    pathSegment: 'skills',
    titleKey: 'console.kind.skill.title',
    title: 'Skills',
    descKey: 'console.kind.skill.desc',
    desc:
      'Skill bundles loaded by the skill registry — packaged prompts + tool allow-lists + few-shot context that the agent can adopt on demand. A skill is a reusable persona/playbook, not a runnable tool; chat surfaces select them at turn start.',
  },
  extension: {
    icon: Layers,
    pathSegment: 'extensions',
    titleKey: 'console.kind.extension.title',
    title: 'Extensions',
    descKey: 'console.kind.extension.desc',
    desc:
      'Installed extensions — out-of-process supervisors that contribute tools, nodes, tables, UI panels, etc. This view is a summary projection; for lifecycle (start/stop/restart) and live state use /extensions.',
  },
}

const KINDS: RegistryKind[] = [
  'tool', 'node', 'rule', 'template', 'table', 'skill', 'extension',
]

// ---------- data hooks --------------------------------------------

function useKindList(kind: RegistryKind) {
  const client = useRubixClient()
  return useQuery<Page<RegistryItem>, Error>({
    queryKey: ['admin', 'registry', kind, 'list'],
    queryFn: async () => {
      const url = `${client.starter.baseUrl}/api/v1/admin/registry?kinds=${kind}`
      const res = await client.starter.fetch(url, {
        method: 'GET',
        headers: { accept: 'application/json' },
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const body = (await res.json()) as Record<string, Page<RegistryItem>>
      return body[kind] ?? { items: [] }
    },
  })
}

function useItemDetail(kind: RegistryKind, id: string | null) {
  const client = useRubixClient()
  return useQuery<RegistryItem, Error>({
    queryKey: ['admin', 'registry', kind, 'detail', id],
    enabled: !!id,
    queryFn: async () => {
      const seg = KIND_META[kind].pathSegment
      const url = `${client.starter.baseUrl}/api/v1/admin/registry/${seg}/${encodeURIComponent(id!)}`
      const res = await client.starter.fetch(url, {
        method: 'GET',
        headers: { accept: 'application/json' },
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      return (await res.json()) as RegistryItem
    },
  })
}

// ---------- invoke panel (tools only) -----------------------------

interface InvokeOutcome {
  status: number
  latencyMs?: number
  body: unknown
  /** Frames captured from `/invoke/stream` SSE, when streamed. */
  frames?: string[]
}

function InvokePanel({
  toolId,
  inputSchema,
}: {
  toolId: string
  inputSchema?: unknown
}) {
  const client = useRubixClient()
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  // A schema is "form-able" if it's a non-empty JSON Schema object
  // (anything beyond `{}` / `{ additionalProperties: ... }`).
  const schema = useMemo<RJSFSchema | null>(() => {
    if (!inputSchema || typeof inputSchema !== 'object') return null
    const keys = Object.keys(inputSchema as Record<string, unknown>)
    const meaningful = keys.filter((k) => k !== 'additionalProperties')
    return meaningful.length > 0 ? (inputSchema as RJSFSchema) : null
  }, [inputSchema])

  const [tenant, setTenant] = useState('default')
  const [mode, setMode] = useState<'form' | 'json'>(schema ? 'form' : 'json')
  const [formData, setFormData] = useState<unknown>({})
  const [inputText, setInputText] = useState('{}')
  const [stream, setStream] = useState(false)
  const [busy, setBusy] = useState(false)
  const [outcome, setOutcome] = useState<InvokeOutcome | null>(null)
  const [parseError, setParseError] = useState<string | null>(null)

  // Reset form state when the tool (and thus schema) changes.
  useEffect(() => {
    setMode(schema ? 'form' : 'json')
    setFormData({})
    setInputText('{}')
    setOutcome(null)
    setParseError(null)
  }, [toolId, schema])

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    setParseError(null)
    let input: unknown = {}
    if (mode === 'form' && schema) {
      input = formData ?? {}
    } else if (inputText.trim().length > 0) {
      try {
        input = JSON.parse(inputText)
      } catch (err) {
        setParseError((err as Error).message)
        return
      }
    }
    setBusy(true)
    setOutcome(null)
    const t0 = performance.now()
    try {
      const segment = stream ? 'invoke/stream' : 'invoke'
      const url = `${client.starter.baseUrl}/api/v1/admin/registry/tools/${encodeURIComponent(toolId)}/${segment}`
      const res = await client.starter.fetch(url, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          accept: stream ? 'text/event-stream' : 'application/json',
          ...readCsrfHeader(),
        },
        body: JSON.stringify({ tenant, input }),
      })
      if (stream && res.body) {
        const frames: string[] = []
        const reader = res.body.getReader()
        const decoder = new TextDecoder('utf-8')
        let buf = ''
        // eslint-disable-next-line no-constant-condition
        while (true) {
          const { value, done } = await reader.read()
          if (done) break
          buf += decoder.decode(value, { stream: true })
          let idx
          while ((idx = buf.indexOf('\n\n')) !== -1) {
            const raw = buf.slice(0, idx).trim()
            buf = buf.slice(idx + 2)
            if (raw) frames.push(raw)
          }
        }
        const last = frames[frames.length - 1] ?? ''
        setOutcome({
          status: res.status,
          latencyMs: Math.round(performance.now() - t0),
          body: last,
          frames,
        })
      } else {
        const ct = res.headers.get('content-type') ?? ''
        const body = ct.includes('json') ? await res.json() : await res.text()
        setOutcome({
          status: res.status,
          latencyMs: Math.round(performance.now() - t0),
          body,
        })
      }
    } catch (err) {
      setOutcome({ status: 0, body: { error: (err as Error).message } })
    } finally {
      setBusy(false)
    }
  }

  return (
    <form onSubmit={onSubmit} className="space-y-3">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_auto_auto]">
        <label className="block">
          <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {tr('console.invoke.tenant', 'Tenant')}
          </span>
          <input
            value={tenant}
            onChange={(e) => setTenant(e.target.value)}
            className="mt-1 w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-sm"
            placeholder="default"
            required
          />
        </label>
        <label className="flex items-end gap-2 pb-1 text-sm">
          <input
            type="checkbox"
            checked={stream}
            onChange={(e) => setStream(e.target.checked)}
          />
          <Radio className="h-4 w-4" />
          {tr('console.invoke.stream', 'Stream (SSE)')}
        </label>
        <div className="flex items-end">
          <Button type="submit" disabled={busy} className="gap-2">
            <Play className="h-4 w-4" />
            {busy ? tr('console.invoke.running', 'Running…') : tr('console.invoke.run', 'Invoke')}
          </Button>
        </div>
      </div>
      <div className="block">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {mode === 'form'
              ? tr('console.invoke.inputForm', 'Input')
              : tr('console.invoke.input', 'Input JSON')}
          </span>
          {schema ? (
            <div className="inline-flex rounded-full ring-1 ring-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 p-0.5 text-[11px]">
              <button
                type="button"
                onClick={() => setMode('form')}
                className={`rounded-full px-2 py-0.5 ${
                  mode === 'form'
                    ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)]'
                    : 'text-[color:var(--color-muted)]'
                }`}
              >
                {tr('console.invoke.modeForm', 'Form')}
              </button>
              <button
                type="button"
                onClick={() => {
                  // Carry the current form value into JSON mode so the user
                  // can tweak free-form fields the renderer doesn't expose.
                  setInputText(JSON.stringify(formData ?? {}, null, 2))
                  setMode('json')
                }}
                className={`rounded-full px-2 py-0.5 ${
                  mode === 'json'
                    ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)]'
                    : 'text-[color:var(--color-muted)]'
                }`}
              >
                {tr('console.invoke.modeJson', 'JSON')}
              </button>
            </div>
          ) : null}
        </div>
        {mode === 'form' && schema ? (
          <div className="rjsf-host mt-1 rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-2">
            <Form
              schema={schema}
              validator={validator}
              formData={formData}
              onChange={(e) => setFormData(e.formData)}
              liveValidate
              showErrorList={false}
              tagName="div"
              uiSchema={{ 'ui:submitButtonOptions': { norender: true } }}
            />
          </div>
        ) : (
          <>
            <textarea
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              rows={6}
              spellCheck={false}
              className="mt-1 w-full rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] px-3 py-2 font-mono text-xs"
            />
            {parseError ? (
              <span className="mt-1 block text-xs text-red-400">
                {tr('console.invoke.parseError', 'Invalid JSON')}: {parseError}
              </span>
            ) : null}
          </>
        )}
      </div>
      {outcome ? <OutcomeView outcome={outcome} /> : null}
    </form>
  )
}

function OutcomeView({ outcome }: { outcome: InvokeOutcome }) {
  const ok = outcome.status >= 200 && outcome.status < 300
  return (
    <div className="rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 p-3">
      <div className="mb-2 flex items-center gap-2 text-xs">
        <span
          className={`inline-flex items-center rounded-full px-2 py-0.5 font-medium ${
            ok ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)]' : 'bg-red-500/15 text-red-400'
          }`}
        >
          HTTP {outcome.status || 'ERR'}
        </span>
        {outcome.latencyMs != null ? (
          <span className="text-[color:var(--color-subtle)]">{outcome.latencyMs} ms</span>
        ) : null}
      </div>
      {outcome.frames ? (
        <div className="space-y-1">
          {outcome.frames.map((f, i) => (
            <pre
              key={i}
              className="overflow-x-auto whitespace-pre-wrap break-all rounded bg-[color:var(--color-bg)] px-2 py-1 font-mono text-[11px]"
            >
              {f}
            </pre>
          ))}
        </div>
      ) : (
        <pre className="overflow-x-auto whitespace-pre-wrap break-all rounded bg-[color:var(--color-bg)] px-2 py-1 font-mono text-[11px]">
          {typeof outcome.body === 'string' ? outcome.body : JSON.stringify(outcome.body, null, 2)}
        </pre>
      )}
    </div>
  )
}

// ---------- main panel --------------------------------------------

function ConsolePanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const [kind, setKind] = useState<RegistryKind>('tool')
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const list = useKindList(kind)
  const detail = useItemDetail(kind, selectedId)

  // Reset selection when switching kind.
  useEffect(() => {
    setSelectedId(null)
  }, [kind])

  const items = useMemo(() => list.data?.items ?? [], [list.data])

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-6">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            {tr('console.eyebrow', 'Admin')}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {tr('console.title', 'Test console')}
        </h1>
        <p className="mt-2 max-w-2xl text-sm text-[color:var(--color-muted)]">
          {tr(
            'console.subtitle',
            'Browse every registry the agent projects and fire a tool against any tenant. Backed by /api/v1/admin/registry.',
          )}
        </p>
      </header>

      {/* kind picker */}
      <div className="mb-3 flex flex-wrap gap-2">
        {KINDS.map((k) => {
          const Icon = KIND_META[k].icon
          const active = k === kind
          return (
            <button
              key={k}
              type="button"
              onClick={() => setKind(k)}
              className={`inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-medium ring-1 transition ${
                active
                  ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)] ring-[color:var(--color-leaf)]/30'
                  : 'bg-[color:var(--color-surface-2)]/40 text-[color:var(--color-muted)] ring-[color:var(--color-border)] hover:text-[color:var(--color-text)]'
              }`}
            >
              <Icon className="h-3.5 w-3.5" />
              {k}
              {active && list.data?.items?.length != null ? (
                <span className="ml-1 text-[color:var(--color-subtle)]">{list.data.items.length}</span>
              ) : null}
            </button>
          )
        })}
      </div>

      {/* per-kind description */}
      <div className="glass mb-6 rounded-2xl px-4 py-3">
        <div className="flex items-center gap-2">
          {(() => {
            const Icon = KIND_META[kind].icon
            return <Icon className="h-4 w-4 text-[color:var(--color-leaf)]" />
          })()}
          <h2 className="text-sm font-semibold tracking-tight">
            {tr(KIND_META[kind].titleKey, KIND_META[kind].title)}
          </h2>
        </div>
        <p className="mt-1 text-xs leading-relaxed text-[color:var(--color-muted)]">
          {tr(KIND_META[kind].descKey, KIND_META[kind].desc)}
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[320px_1fr]">
        {/* item list */}
        <aside className="glass rounded-2xl p-2">
          {list.isLoading ? (
            <div className="space-y-2 p-2">
              <Skeleton className="h-6 w-full" />
              <Skeleton className="h-6 w-full" />
              <Skeleton className="h-6 w-full" />
            </div>
          ) : list.error ? (
            <p className="px-3 py-2 text-xs text-red-400">{String(list.error.message)}</p>
          ) : items.length === 0 ? (
            <p className="px-3 py-2 text-xs text-[color:var(--color-subtle)]">
              {tr('console.list.empty', 'No items.')}
            </p>
          ) : (
            <ul className="max-h-[70vh] overflow-y-auto">
              {items.map((it) => {
                const active = it.id === selectedId
                return (
                  <li key={it.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(it.id)}
                      className={`block w-full rounded-md px-3 py-2 text-left text-sm transition ${
                        active
                          ? 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-text)]'
                          : 'hover:bg-[color:var(--color-surface-2)]/60'
                      }`}
                    >
                      <div className="font-mono text-xs">{it.id}</div>
                      {it.label && it.label !== it.id ? (
                        <div className="text-[11px] text-[color:var(--color-subtle)]">{it.label}</div>
                      ) : null}
                    </button>
                  </li>
                )
              })}
            </ul>
          )}
        </aside>

        {/* detail */}
        <main className="glass min-h-[60vh] rounded-2xl p-5">
          {!selectedId ? (
            <p className="text-sm text-[color:var(--color-subtle)]">
              {tr('console.detail.pick', 'Pick an item on the left to inspect.')}
            </p>
          ) : detail.isLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-6 w-1/2" />
              <Skeleton className="h-24 w-full" />
            </div>
          ) : detail.error ? (
            <p className="text-sm text-red-400">{String(detail.error.message)}</p>
          ) : detail.data ? (
            <DetailView kind={kind} item={detail.data} />
          ) : null}
        </main>
      </div>
    </section>
  )
}

function DetailView({ kind, item }: { kind: RegistryKind; item: RegistryItem }) {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const sourceLabel =
    item.source.kind === 'extension' && item.source.id
      ? `extension:${item.source.id}`
      : item.source.kind

  return (
    <div className="space-y-5">
      <header>
        <div className="font-mono text-xs text-[color:var(--color-subtle)]">{item.id}</div>
        <h2 className="mt-1 text-2xl font-medium tracking-[-0.02em]">{item.label}</h2>
        {item.summary ? (
          <p className="mt-1 text-sm text-[color:var(--color-muted)]">{item.summary}</p>
        ) : null}
        <div className="mt-2 inline-flex items-center rounded-full bg-[color:var(--color-surface-2)]/60 px-2 py-0.5 text-[11px] uppercase tracking-wider text-[color:var(--color-subtle)] ring-1 ring-[color:var(--color-border)]">
          {sourceLabel}
        </div>
      </header>

      {kind === 'tool' ? (
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {tr('console.detail.invoke', 'Invoke')}
          </h3>
          <InvokePanel toolId={item.id} inputSchema={item.input_schema} />
        </section>
      ) : null}

      {item.input_schema ? (
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {tr('console.detail.inputSchema', 'Input schema')}
          </h3>
          <pre className="overflow-x-auto rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-bg)] p-3 font-mono text-[11px]">
            {JSON.stringify(item.input_schema, null, 2)}
          </pre>
        </section>
      ) : null}

      {item.metadata && Object.keys(item.metadata).length > 0 ? (
        <section>
          <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {tr('console.detail.metadata', 'Metadata')}
          </h3>
          <pre className="overflow-x-auto rounded-lg border border-[color:var(--color-border)] bg-[color:var(--color-bg)] p-3 font-mono text-[11px]">
            {JSON.stringify(item.metadata, null, 2)}
          </pre>
        </section>
      ) : null}
    </div>
  )
}

function ConsoleRoute() {
  return (
    <ErrorBoundary>
      <ConsolePanel />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/console')({
  component: ConsoleRoute,
})
