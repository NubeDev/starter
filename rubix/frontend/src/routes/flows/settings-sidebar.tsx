// `<SettingsSidebar>` — per-node configuration panel for `/flows/$flowId`.
//
// Reads the currently-selected node id from the parent route (which
// owns xyflow selection state via `<FlowCanvas reactFlowProps>`),
// looks up its kind in the route-level `flowRegistry`, fetches the
// kind's `config_schema` via `useFlowKinds()`, and renders a minimal
// hand-rolled JSON-Schema form. Only primitive types are first-class
// — `string` / `number` / `integer` / `boolean` / `enum`. Anything
// shaped more interestingly (objects, arrays, oneOf, …) falls back
// to a `<textarea>` of raw JSON with parse-error feedback per the
// stage E.3 SCOPE.
//
// The Save button serialises the updated YAML (current body with
// the selected node's `config` swapped) and dispatches
// `flowDeploy({ flow_id, body_yaml })`. Because the engine
// classifier short-circuits same-topology revisions to slot writes,
// a save here lands as a Settings hot-reload without restarting any
// running node. If `flowDeploy` rejects with a revision-moved
// conflict the sidebar surfaces an inline error toast.

import { useEffect, useMemo, useState } from 'react'
import { useIntl } from 'react-intl'
import * as YAML from 'yaml'
import {
  Button,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Skeleton,
  Switch,
  Textarea,
} from '@nube/starter-ui-kit'
import { useFlowDeploy, useFlowKinds } from '@nube/rubix-client-react'
import type { FlowKindItem } from '@nube/rubix-client-ts'

/** Minimal JSON-Schema shape we walk. Permissive — unknown fields ignored. */
interface JsonSchemaProperty {
  type?: string | string[]
  enum?: Array<string | number | boolean>
  description?: string
  default?: unknown
}
interface JsonSchemaObject {
  type?: string
  properties?: Record<string, JsonSchemaProperty>
  required?: string[]
}

export interface SettingsSidebarProps {
  flowId: string
  selectedNodeId: string | null
  /** Current YAML body of the live revision; the source of truth we mutate. */
  bodyYaml: string
  /** Called after a successful deploy so the parent can refresh local state. */
  onSaved?: (newBodyYaml: string) => void
}

interface NodeConfigShape {
  id: string
  kind: string
  config: Record<string, unknown>
}

/** Pull `{id, kind, config}` for the selected node out of a parsed body. */
function findNode(doc: YAML.Document.Parsed | null, nodeId: string): NodeConfigShape | null {
  if (!doc) return null
  const json = doc.toJS() as { nodes?: Array<{ id: string; kind: string; config?: unknown }> }
  const found = json.nodes?.find((n) => n.id === nodeId)
  if (!found) return null
  const cfg = (found.config ?? {}) as Record<string, unknown>
  return { id: found.id, kind: found.kind, config: cfg }
}

/**
 * Replace `nodes[i].config` in-place on a parsed YAML document.
 * Preserves the rest of the document (comments, ordering, other
 * nodes) so the round-trip body stays diff-friendly.
 */
function replaceNodeConfig(
  doc: YAML.Document.Parsed,
  nodeId: string,
  newConfig: Record<string, unknown>,
): void {
  const nodesNode = doc.get('nodes', true) as YAML.YAMLSeq | undefined
  if (!nodesNode || !YAML.isSeq(nodesNode)) return
  for (const entry of nodesNode.items) {
    if (!YAML.isMap(entry)) continue
    const id = entry.get('id')
    if (id !== nodeId) continue
    entry.set('config', newConfig)
    return
  }
}

/** True if every property type is one we can render as a first-class control. */
function isSimpleSchema(schema: JsonSchemaObject | undefined): schema is JsonSchemaObject {
  if (!schema || schema.type !== 'object' || !schema.properties) return false
  return Object.values(schema.properties).every((p) => {
    if (p.enum) return true
    const t = Array.isArray(p.type) ? p.type[0] : p.type
    return t === 'string' || t === 'number' || t === 'integer' || t === 'boolean'
  })
}

/** Coerce form-string input back to the schema's primitive type. */
function coerce(value: string, prop: JsonSchemaProperty): unknown {
  const t = Array.isArray(prop.type) ? prop.type[0] : prop.type
  if (prop.enum) return prop.enum.find((v) => String(v) === value) ?? value
  if (t === 'number' || t === 'integer') {
    const n = Number(value)
    return Number.isFinite(n) ? n : value
  }
  if (t === 'boolean') return value === 'true'
  return value
}

function PrimitiveField(props: {
  name: string
  prop: JsonSchemaProperty
  value: unknown
  onChange(v: unknown): void
}) {
  const { name, prop, value, onChange } = props
  const id = `setting-${name}`
  const stringValue = value === undefined || value === null ? '' : String(value)
  const t = Array.isArray(prop.type) ? prop.type[0] : prop.type

  if (prop.enum) {
    return (
      <div className="space-y-1.5">
        <Label htmlFor={id}>{name}</Label>
        <Select value={stringValue} onValueChange={(v) => onChange(coerce(v, prop))}>
          <SelectTrigger id={id}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {prop.enum.map((opt) => (
              <SelectItem key={String(opt)} value={String(opt)}>
                {String(opt)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {prop.description ? (
          <p className="text-xs text-[color:var(--color-muted)]">{prop.description}</p>
        ) : null}
      </div>
    )
  }

  if (t === 'boolean') {
    return (
      <div className="flex items-start justify-between gap-3">
        <div>
          <Label htmlFor={id}>{name}</Label>
          {prop.description ? (
            <p className="text-xs text-[color:var(--color-muted)]">{prop.description}</p>
          ) : null}
        </div>
        <Switch id={id} checked={value === true} onCheckedChange={(v) => onChange(Boolean(v))} />
      </div>
    )
  }

  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{name}</Label>
      <Input
        id={id}
        type={t === 'number' || t === 'integer' ? 'number' : 'text'}
        value={stringValue}
        onChange={(e) => onChange(coerce(e.target.value, prop))}
      />
      {prop.description ? (
        <p className="text-xs text-[color:var(--color-muted)]">{prop.description}</p>
      ) : null}
    </div>
  )
}

/** Render the per-property primitive form for a simple object schema. */
function SimpleForm(props: {
  schema: JsonSchemaObject
  config: Record<string, unknown>
  onChange(next: Record<string, unknown>): void
}) {
  const { schema, config, onChange } = props
  return (
    <div className="space-y-4">
      {Object.entries(schema.properties ?? {}).map(([name, prop]) => (
        <PrimitiveField
          key={name}
          name={name}
          prop={prop}
          value={config[name] ?? prop.default}
          onChange={(v) => onChange({ ...config, [name]: v })}
        />
      ))}
    </div>
  )
}

/** Raw-JSON fallback for non-trivial schemas. */
function JsonFallback(props: {
  config: Record<string, unknown>
  onChange(next: Record<string, unknown>): void
}) {
  const { config, onChange } = props
  const [draft, setDraft] = useState(() => JSON.stringify(config, null, 2))
  const [error, setError] = useState<string | null>(null)

  // Reset draft when the underlying config switches (different node).
  useEffect(() => {
    setDraft(JSON.stringify(config, null, 2))
    setError(null)
  }, [config])

  return (
    <div className="space-y-2">
      <Label htmlFor="raw-json">config (JSON)</Label>
      <Textarea
        id="raw-json"
        rows={12}
        value={draft}
        onChange={(e) => {
          setDraft(e.target.value)
          try {
            const parsed = JSON.parse(e.target.value)
            setError(null)
            if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
              onChange(parsed as Record<string, unknown>)
            }
          } catch (err) {
            setError(err instanceof Error ? err.message : String(err))
          }
        }}
        className="font-mono text-xs"
      />
      {error ? <p className="text-xs text-red-500">{error}</p> : null}
    </div>
  )
}

export function SettingsSidebar({
  flowId,
  selectedNodeId,
  bodyYaml,
  onSaved,
}: SettingsSidebarProps) {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const kinds = useFlowKinds()
  const deploy = useFlowDeploy()

  // Re-parse YAML whenever the body changes — cheap and keeps the
  // form synced with the most recently saved revision.
  const doc = useMemo<YAML.Document.Parsed | null>(() => {
    if (!bodyYaml) return null
    try {
      return YAML.parseDocument(bodyYaml)
    } catch {
      return null
    }
  }, [bodyYaml])

  const node = useMemo(
    () => (selectedNodeId ? findNode(doc, selectedNodeId) : null),
    [doc, selectedNodeId],
  )

  const kind = useMemo<FlowKindItem | undefined>(
    () => kinds.data?.kinds.find((k) => k.kind_id === node?.kind),
    [kinds.data, node?.kind],
  )

  const schema = (kind?.config_schema ?? undefined) as JsonSchemaObject | undefined
  const [draftConfig, setDraftConfig] = useState<Record<string, unknown>>({})
  const [conflictMsg, setConflictMsg] = useState<string | null>(null)

  useEffect(() => {
    setDraftConfig(node?.config ?? {})
    setConflictMsg(null)
  }, [node?.id, node?.kind, bodyYaml])

  if (!selectedNodeId) {
    return (
      <aside className="rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4 text-sm text-[color:var(--color-muted)]">
        {tr(
          'flows.settings.empty',
          'Select a node in the canvas to edit its settings.',
        )}
      </aside>
    )
  }

  if (!node) {
    return (
      <aside className="rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4 text-sm text-[color:var(--color-muted)]">
        {tr('flows.settings.notFound', 'Selected node not found in deployed YAML.')}
      </aside>
    )
  }

  const handleSave = async () => {
    if (!doc) return
    setConflictMsg(null)
    replaceNodeConfig(doc, node.id, draftConfig)
    const nextYaml = doc.toString()
    try {
      await deploy.mutateAsync({ flow_id: flowId, body_yaml: nextYaml })
      onSaved?.(nextYaml)
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      setConflictMsg(msg)
    }
  }

  return (
    <aside className="space-y-4 rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4">
      <header className="space-y-1">
        <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[color:var(--color-leaf)]">
          {tr('flows.settings.eyebrow', 'Node settings')}
        </p>
        <h2 className="text-base font-medium">{node.id}</h2>
        <p className="font-mono text-xs text-[color:var(--color-muted)]">{node.kind}</p>
      </header>

      {kinds.isLoading ? (
        <Skeleton className="h-32 w-full" />
      ) : !schema ? (
        <p className="text-xs text-[color:var(--color-muted)]">
          {tr(
            'flows.settings.noSchema',
            'No schema published for this kind — falling back to raw JSON.',
          )}
        </p>
      ) : null}

      {schema && isSimpleSchema(schema) ? (
        <SimpleForm schema={schema} config={draftConfig} onChange={setDraftConfig} />
      ) : (
        <JsonFallback config={draftConfig} onChange={setDraftConfig} />
      )}

      {conflictMsg ? (
        <div className="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-500">
          {conflictMsg}
        </div>
      ) : null}

      <div className="flex justify-end">
        <Button onClick={handleSave} disabled={deploy.isPending} size="sm">
          {deploy.isPending
            ? tr('flows.settings.saving', 'Saving…')
            : tr('flows.settings.save', 'Save')}
        </Button>
      </div>
    </aside>
  )
}
