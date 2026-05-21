/**
 * Authoring contract for built-in SDUI components.
 *
 * One `ComponentSpec` is co-located with each component file
 * (`components/<X>.tsx` exports `<x>Spec`); the `index.ts` barrel
 * aggregates them into `builtinComponentRegistry`. The runtime
 * `Renderer` and the (future) builder palette both read the same
 * spec map — one source of truth, no parallel switch statements.
 *
 * The `Kind` union below is the wire-format discriminant the IR
 * emits as `{ "type": ... }`; the drift test is the gate that this
 * stays in sync with `starter-ui-ir`'s `Component` enum.
 */
import type { ComponentType } from "react";

export type Kind =
  | "page"
  | "row"
  | "col"
  | "grid"
  | "stack"
  | "tabs"
  | "card"
  | "text"
  | "heading"
  | "badge"
  | "kpi"
  | "kpi_grid"
  | "button"
  | "link"
  | "table"
  | "form"
  | "field"
  | "select"
  | "toggle"
  | "custom";

export type ComponentRenderProps<TNode> = { node: TNode };

export interface ComponentSpec<TNode = unknown> {
  kind: Kind;
  Component: ComponentType<ComponentRenderProps<TNode>>;
}

export type ComponentRegistry = Readonly<
  Partial<Record<Kind, ComponentSpec<unknown>>>
>;
