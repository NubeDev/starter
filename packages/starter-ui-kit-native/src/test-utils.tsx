// Shared test helpers. Renders a primitive with `@testing-library/react`
// under the host-element RN mock (see `vitest.config.ts` alias →
// `src/__mocks__/react-native.tsx`) and exposes accessors for the
// a11y props the kit promises to wire through.

import { render } from "@testing-library/react";
import * as React from "react";

export function mount(node: React.ReactElement): HTMLElement {
  return render(node).container;
}

/** Find the first host element rendered with a `data-slot` value. */
export function bySlot(root: HTMLElement, slot: string): Element | null {
  return root.querySelector(`[data-slot="${slot}"]`);
}

/** Read an a11y attribute the kit forwarded onto the RN host. The
 * host-element mock copies prop keys verbatim onto the DOM, so
 * `accessibilityRole="button"` becomes `accessibilityrole="button"`
 * (HTML attrs are case-insensitive — jsdom lowercases). */
export function a11y(el: Element | null, key: string): string | null {
  if (!el) return null;
  return el.getAttribute(key.toLowerCase());
}
