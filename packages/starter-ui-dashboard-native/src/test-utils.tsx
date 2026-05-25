// Shared test helpers. Each widget test mounts under jsdom with
// `@nube/starter-ui-kit-native`, `react-native-svg`, and `moti`
// resolved to the host-element mocks in `src/__mocks__/`.

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

export function bySvg(root: HTMLElement, tag: string): Element | null {
  return root.querySelector(`[data-svg="${tag}"]`);
}

export function allBySvg(root: HTMLElement, tag: string): Element[] {
  return Array.from(root.querySelectorAll(`[data-svg="${tag}"]`));
}

export function byMoti(root: HTMLElement, tag: string): Element | null {
  return root.querySelector(`[data-moti="${tag}"]`);
}
