// Pure helper: append one node of the given kind to a flow YAML
// body. Used by both the click-to-add path in `<NodePalette>` and
// the drag-drop path in `/flows/$flowId`. Centralising the YAML
// mutation keeps default-config seeding, unique-id allocation, and
// short-id derivation in one place.

import * as YAML from 'yaml'
import type { FlowKindItem } from '@nube/rubix-client-ts'

/** Permissive JSON-Schema shape we walk to seed default config. */
interface SchemaProp {
  type?: string | string[]
  enum?: Array<string | number | boolean>
  default?: unknown
}
interface SchemaShape {
  type?: string
  properties?: Record<string, SchemaProp>
}

function defaultForProp(prop: SchemaProp): unknown {
  if (prop.default !== undefined) return prop.default
  if (prop.enum && prop.enum.length > 0) return prop.enum[0]
  const t = Array.isArray(prop.type) ? prop.type[0] : prop.type
  switch (t) {
    case 'number':
    case 'integer':
      return 0
    case 'boolean':
      return false
    case 'array':
      return []
    case 'object':
      return {}
    default:
      return ''
  }
}

function defaultConfig(schema: SchemaShape | undefined): Record<string, unknown> {
  if (!schema || schema.type !== 'object' || !schema.properties) return {}
  const out: Record<string, unknown> = {}
  for (const [key, prop] of Object.entries(schema.properties)) {
    out[key] = defaultForProp(prop)
  }
  return out
}

function uniqueNodeId(base: string, existing: Set<string>): string {
  if (!existing.has(base)) return base
  let n = 2
  while (existing.has(`${base}-${n}`)) n += 1
  return `${base}-${n}`
}

/**
 * Reverse-DNS `kind_id` (`starter.flow.trigger.schedule`) -> a short,
 * pleasant short id stem the operator sees on the canvas
 * (`trigger-schedule`). The full kind id stays on the node's `kind:`
 * field; this is just the human-readable seed for `id:`.
 */
function shortIdStem(kindId: string): string {
  const last = kindId.split('.').slice(-2).join('-')
  return last.replace(/[^a-z0-9-]/gi, '-').toLowerCase() || 'node'
}

/** Read the existing `nodes[].id` set out of a YAML body. */
export function existingNodeIds(bodyYaml: string): Set<string> {
  if (!bodyYaml) return new Set()
  try {
    const parsed = YAML.parse(bodyYaml) as {
      nodes?: Array<{ id?: string }>
    } | null
    return new Set(
      parsed?.nodes?.map((n) => n.id).filter((s): s is string => !!s) ?? [],
    )
  } catch {
    return new Set()
  }
}

/**
 * Append one `{ id, kind, config }` node entry to `bodyYaml` and
 * return the new YAML body. Throws on YAML parse failure (caller
 * surfaces the error). Comments / formatting / other nodes are
 * preserved via `YAML.parseDocument`'s round-trip.
 */
export function appendFlowNode(bodyYaml: string, kind: FlowKindItem): string {
  const doc = YAML.parseDocument(bodyYaml)
  let nodes = doc.get('nodes', true) as YAML.YAMLSeq | undefined
  if (!nodes || !YAML.isSeq(nodes)) {
    nodes = new YAML.YAMLSeq()
    doc.set('nodes', nodes)
  }
  const id = uniqueNodeId(shortIdStem(kind.kind_id), existingNodeIds(bodyYaml))
  const newNode = doc.createNode({
    id,
    kind: kind.kind_id,
    config: defaultConfig(kind.config_schema as SchemaShape | undefined),
  }) as YAML.YAMLMap
  nodes.add(newNode)
  return doc.toString()
}

/**
 * MIME type the palette writes on drag-start and the canvas reads
 * on drop. Carries the node `kind_id` as a plain string.
 */
export const FLOW_KIND_DRAG_MIME = 'application/x-rubix-flow-kind'
