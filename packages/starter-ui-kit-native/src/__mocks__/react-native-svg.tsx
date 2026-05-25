// Vitest-only shim for `react-native-svg`. Renders as inline SVG host
// elements under jsdom.

import * as React from "react";

type AnyProps = Record<string, unknown> & { children?: React.ReactNode };

function svg(tag: string) {
  return function SvgEl(props: AnyProps) {
    const { children, ...rest } = props;
    return React.createElement(`svg-${tag}`, { ...rest, "data-svg": tag }, children);
  };
}

export const Svg = svg("svg");
export const Circle = svg("circle");
export const Rect = svg("rect");
export const Path = svg("path");
export const G = svg("g");
export const Line = svg("line");
export const Polyline = svg("polyline");
export const Defs = svg("defs");
export const LinearGradient = svg("linear-gradient");
export const Stop = svg("stop");
export default Svg;
