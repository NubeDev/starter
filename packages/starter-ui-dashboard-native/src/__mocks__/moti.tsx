// Vitest-only shim for `moti`. `MotiView` renders as a plain host
// element under jsdom; the `from`/`animate`/`transition`/`exit` props
// are preserved on the DOM node so tests can verify the widget passed
// them through unchanged.

import * as React from "react";

type AnyProps = Record<string, unknown> & {
  children?: React.ReactNode;
  style?: unknown;
};

function moti(tag: string) {
  return function MotiEl(props: AnyProps) {
    const { children, style, from, animate, transition, exit, ...rest } = props;
    return React.createElement(
      `moti-${tag}`,
      {
        ...rest,
        "data-moti": tag,
        "data-style": JSON.stringify(style ?? null),
        "data-from": JSON.stringify(from ?? null),
        "data-animate": JSON.stringify(animate ?? null),
        "data-transition": JSON.stringify(transition ?? null),
        "data-exit": JSON.stringify(exit ?? null),
      },
      children,
    );
  };
}

export const MotiView = moti("view");
export const MotiText = moti("text");

export const AnimatePresence: React.FC<{ children?: React.ReactNode }> = ({
  children,
}) => React.createElement(React.Fragment, null, children);
