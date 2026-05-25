// Vitest-only shim for `moti`. `MotiView` is rendered as a plain host
// element under jsdom; the `from`/`animate`/`transition`/`exit` props
// are preserved on the DOM node so a snapshot can verify the kit
// passed them through unchanged.

import * as React from "react";

type AnyProps = Record<string, unknown> & {
  children?: React.ReactNode;
  style?: unknown;
};

function moti(tag: string) {
  return function MotiEl(props: AnyProps) {
    const { children, style, ...rest } = props;
    return React.createElement(
      `moti-${tag}`,
      { ...rest, "data-moti": tag, "data-style": JSON.stringify(style ?? null) },
      children,
    );
  };
}

export const MotiView = moti("view");
export const MotiText = moti("text");

export const AnimatePresence: React.FC<{ children?: React.ReactNode }> = ({
  children,
}) => React.createElement(React.Fragment, null, children);
