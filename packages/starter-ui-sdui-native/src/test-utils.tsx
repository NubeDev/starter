// Shared test helpers. Each renderer test mounts under jsdom with
// the kit alias resolving to `src/__mocks__/starter-ui-kit-native.tsx`.

import { render } from "@testing-library/react";
import * as React from "react";

export function mount(node: React.ReactElement): HTMLElement {
  return render(node).container;
}

export function byKit(root: HTMLElement, kit: string): Element | null {
  return root.querySelector(`[data-kit="${kit}"]`);
}

export function allByKit(root: HTMLElement, kit: string): Element[] {
  return Array.from(root.querySelectorAll(`[data-kit="${kit}"]`));
}
