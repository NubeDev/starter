/**
 * Tree-patch helpers — used by optimistic action hints and authoritative
 * Patch / FullRender responses. Walk the tree, find the node with the
 * matching `id`, and shallow-merge fields (or swap the subtree).
 * Everything else stays pointer-identical so React only re-renders the
 * affected branch.
 */
import type { UiComponent, UiComponentTree } from "./types.js";

export function mergeAt(
  tree: UiComponentTree,
  targetId: string,
  fields: Record<string, unknown>,
): UiComponentTree {
  return { ...tree, root: mergeNode(tree.root, targetId, fields) };
}

export function replaceAt(
  tree: UiComponentTree,
  targetId: string,
  replacement: UiComponent,
): UiComponentTree {
  return { ...tree, root: replaceNode(tree.root, targetId, replacement) };
}

function mergeNode(
  node: UiComponent,
  targetId: string,
  fields: Record<string, unknown>,
): UiComponent {
  if (node.id === targetId) {
    return { ...node, ...fields };
  }
  const children = node.children;
  if (Array.isArray(children)) {
    const next = children.map((c) => mergeNode(c, targetId, fields));
    if (next.some((c, i) => c !== children[i])) {
      return { ...node, children: next };
    }
  }
  const tabs = node.tabs;
  if (Array.isArray(tabs)) {
    const nextTabs = tabs.map((t) => ({
      ...t,
      children: t.children.map((c) => mergeNode(c, targetId, fields)),
    }));
    if (nextTabs.some((t, i) => t !== tabs[i])) {
      return { ...node, tabs: nextTabs };
    }
  }
  return node;
}

function replaceNode(
  node: UiComponent,
  targetId: string,
  replacement: UiComponent,
): UiComponent {
  if (node.id === targetId) return replacement;
  const children = node.children;
  if (Array.isArray(children)) {
    const next = children.map((c) => replaceNode(c, targetId, replacement));
    if (next.some((c, i) => c !== children[i])) {
      return { ...node, children: next };
    }
  }
  return node;
}
