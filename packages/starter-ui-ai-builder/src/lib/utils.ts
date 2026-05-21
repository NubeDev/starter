import clsx, { type ClassValue } from "clsx";

export function cn(...inputs: ClassValue[]): string {
  return clsx(inputs);
}

export function makeId(prefix = "id"): string {
  return `${prefix}_${Date.now().toString(36)}_${Math.random()
    .toString(36)
    .slice(2, 8)}`;
}

/** Walk a tree and return true if any node has the given `id`. */
import type { UiComponent } from "@nube/starter-sdui-react";
export function treeHasId(node: UiComponent | undefined, id: string): boolean {
  if (!node) return false;
  if (node.id === id) return true;
  if (Array.isArray(node.children)) {
    for (const c of node.children) {
      if (treeHasId(c, id)) return true;
    }
  }
  if (Array.isArray(node.tabs)) {
    for (const t of node.tabs) {
      for (const c of t.children) {
        if (treeHasId(c, id)) return true;
      }
    }
  }
  return false;
}
