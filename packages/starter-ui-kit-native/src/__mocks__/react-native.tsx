// Vitest-only shim for the `react-native` peer dep.
//
// Renders each RN host element as a custom DOM element under jsdom so
// `@testing-library/react` can drive the primitives and snapshot
// trees. Props pass through unchanged, which is the whole point:
// accessibility prop wiring (`accessibilityRole`, `accessibilityLabel`,
// `accessibilityHint`) is the contract this kit enforces, and the
// tests must be able to read it back from the rendered output.
//
// This file is *not* shipped — see `vitest.config.ts` alias.

import * as React from "react";

type AnyProps = Record<string, unknown> & {
  children?: React.ReactNode;
  style?: unknown;
};

function host(slot: string) {
  return React.forwardRef<unknown, AnyProps>(function HostEl(props, ref) {
    const { children, style, ...rest } = props;
    return React.createElement(
      `rn-${slot}`,
      { ...rest, "data-slot": slot, ref, "data-style": JSON.stringify(style ?? null) },
      children as React.ReactNode,
    );
  });
}

export const View = host("view");
export const Text = host("text");
export const Pressable = host("pressable");
export const TextInput = host("textinput");
export const ScrollView = host("scrollview");
export const ActivityIndicator = host("activity-indicator");

export const Modal: React.FC<AnyProps & { visible?: boolean }> = ({
  visible,
  children,
  ...rest
}) =>
  visible
    ? React.createElement("rn-modal", { ...rest, "data-slot": "modal" }, children)
    : null;

export const StyleSheet = {
  create<T extends Record<string, AnyProps>>(s: T): T {
    return s;
  },
  flatten(s: unknown): Record<string, unknown> {
    if (!s) return {};
    if (Array.isArray(s)) return Object.assign({}, ...s.map((x) => x ?? {}));
    return s as Record<string, unknown>;
  },
  absoluteFillObject: {
    position: "absolute",
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
  },
  hairlineWidth: 1,
};

export const Platform = {
  OS: "ios" as const,
  select<T>(o: { ios?: T; android?: T; web?: T; default?: T }): T | undefined {
    return o.ios ?? o.default;
  },
};

export const Dimensions = {
  get(_: "window" | "screen") {
    return { width: 375, height: 812, scale: 2, fontScale: 1 };
  },
};

export function useColorScheme(): "light" | "dark" {
  return "light";
}

// Touch gesture mock — under jsdom we have no pointer events; tests
// don't drive the slider, they just snapshot its mount.
export const PanResponder = {
  create(_handlers: Record<string, unknown>) {
    return { panHandlers: {} as Record<string, unknown> };
  },
};
