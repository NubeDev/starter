// Bidirectional adapter between the IR `ComponentTree` (the wire
// shape `rubix.dashboard.{get,update}` ships) and Puck's
// `Data` (what the editor canvas operates on).
//
// **IR shape**: `{ ir_version, root: { type: "page", children?, ... } }`.
// **Puck shape**: `{ root: { props: PageOwnProps }, content: PuckNode[], zones? }`
// where each `PuckNode` is `{ type, props: { id, ...irProps, [slotName]: PuckNode[] } }`.
//
// Mapping rules — kept intentionally narrow so the round-trip is
// stable for the v1 author surface:
//
//   1. The IR root is always the `page` variant. Its own props
//      (everything except `type` / `children`) become Puck's
//      `data.root.props`. Its `children` become `data.content`.
//   2. Every other IR node `{ type, ...props }` becomes a Puck
//      node `{ type, props: { id, ...props } }`. Puck requires
//      every node to carry a stable `id` — we synthesise one
//      from the IR node's own `id` if present, else a content-
//      addressed `${type}-${index}-${depth}` fallback.
//   3. For props that the curated `SLOTS` table marks as slots
//      (e.g. `row.children`), the value is `PuckNode[]` — recurse.
//      For all other props we pass through verbatim. Array fields
//      whose items are not IR `Component`s (e.g. `chart.sources`)
//      stay as plain objects — Puck's `array` field renders them
//      as nested rows in the props panel.
//
// The adapter does **not** validate. The schema-drift guard from
// PR1 keeps the curated tables aligned with the IR; anything the
// adapter sees that it doesn't understand passes through
// unchanged so the operator can still see and edit it.

import type { Data } from "@measured/puck";

import { SLOTS, isSlot } from "./curation/slots.js";

// IR node shape — loose because the IR is a tagged union we walk
// generically.
export interface IrNode {
  type: string;
  id?: string;
  children?: IrNode[];
  [key: string]: unknown;
}

export interface ComponentTree {
  ir_version: number;
  root: IrNode;
  vars?: Record<string, unknown>;
}

interface PuckNode {
  type: string;
  props: Record<string, unknown> & { id: string };
}

/** IR_VERSION the adapter emits on the way out. Mirrors the
 *  schemars-emitted ComponentTree.ir_version. Update alongside
 *  the IR crate's `IR_VERSION` const. */
export const IR_VERSION = 5;

/** Stable id generator — falls back to a deterministic synthesised
 *  id when the IR node didn't carry one. Puck panics if any node's
 *  id collides, so the path is woven into the fallback. */
function nodeId(node: IrNode, path: string): string {
  if (typeof node.id === "string" && node.id.length > 0) return node.id;
  return `${node.type}-${path}`;
}

function irToPuckNode(node: IrNode, path: string): PuckNode {
  const { type, id: _id, ...rest } = node;
  void _id;
  const props: Record<string, unknown> = { id: nodeId(node, path) };
  for (const [key, value] of Object.entries(rest)) {
    if (isSlot(type, key) && Array.isArray(value)) {
      props[key] = (value as IrNode[]).map((child, i) =>
        irToPuckNode(child, `${path}.${key}.${i}`),
      );
    } else {
      props[key] = value;
    }
  }
  return { type, props: props as Record<string, unknown> & { id: string } };
}

/** Convert an IR `ComponentTree` into the Puck `Data` shape the
 *  canvas operates on. */
export function componentTreeToPuckData(tree: ComponentTree): Data {
  const root = tree.root ?? { type: "page" };
  if (root.type !== "page") {
    // Per scope the root is always `page`. We don't refuse — wrap
    // the unknown root in a synthetic page so the operator can
    // still open the editor and see what they've got.
    return {
      root: { props: {} },
      content: [irToPuckNode(root, "r")],
    } as Data;
  }
  const { type: _type, id: rootId, children, ...rootRest } = root;
  void _type;
  // Preserve the root id in props so puckDataToComponentTree can
  // restore it. Component::Page.id is required (non-optional) on
  // the server; dropping it makes saved body_json fail to deserialise.
  // Fall back to "root" when the stored tree omits it.
  const rootProps: Record<string, unknown> = { ...rootRest, __root_id: rootId ?? "root" };
  return {
    root: { props: rootProps },
    content: (Array.isArray(children)
      ? children.map((c, i) => irToPuckNode(c, `r.${i}`))
      : []) as unknown as Data["content"],
  } as Data;
}

/** Synthesised ids from `irToPuckNode` are not part of the wire
 *  shape — drop them on the way back. Stable author-supplied ids
 *  flow through. */
function isSynthId(value: unknown, type: string, path: string): boolean {
  return value === `${type}-${path}`;
}

function puckNodeToIr(node: PuckNode, path: string): IrNode {
  const { id, ...rest } = node.props;
  const out: IrNode = { type: node.type };
  if (!isSynthId(id, node.type, path)) {
    out.id = id;
  }
  for (const [key, value] of Object.entries(rest)) {
    if (isSlot(node.type, key) && Array.isArray(value)) {
      out[key] = (value as PuckNode[]).map((child, i) =>
        puckNodeToIr(child, `${path}.${key}.${i}`),
      );
    } else {
      out[key] = value;
    }
  }
  return out;
}

/** Convert Puck's `Data` back into an IR `ComponentTree` suitable
 *  for `rubix.dashboard.update`'s `body_json`. */
export function puckDataToComponentTree(
  data: Data,
  options: { irVersion?: number } = {},
): ComponentTree {
  const content = (data.content ?? []) as PuckNode[];
  const { __root_id: rootId, ...restProps } = (data.root?.props ?? {}) as Record<string, unknown> & { __root_id?: string };
  const root: IrNode = {
    type: "page",
    // Restore the preserved root id so the server can deserialise
    // Component::Page whose id field is required (non-optional).
    ...(rootId !== undefined ? { id: rootId } : { id: "root" }),
    ...restProps,
    children: content.map((c, i) => puckNodeToIr(c, `r.${i}`)),
  };
  return {
    ir_version: options.irVersion ?? IR_VERSION,
    root,
  };
}

// Re-export `SLOTS` consumers may want to consult when extending
// the mapping for non-page roots.
export { SLOTS };
