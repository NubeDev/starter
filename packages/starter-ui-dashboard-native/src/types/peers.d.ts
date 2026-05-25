// Ambient shims for the native peer deps.
//
// The dashboard-native widgets import `react-native-svg`, `moti`, and
// (optionally, for the `ActivityItem.icon` type) `lucide-react`. The
// consuming Expo / RN app installs the real packages; here we keep
// the workspace `tsc --noEmit` and the vitest runs free of the heavy
// native toolchains by declaring loose-but-honest module shapes.
//
// Behaviour under tests is owned by the host-element mocks in
// `src/__mocks__/`, which the vitest config aliases in. These shims
// are types-only.

declare module "react-native-svg" {
  import * as React from "react";
  type AnyProps = Record<string, unknown> & { children?: React.ReactNode };
  export const Svg: React.FC<AnyProps>;
  export const Circle: React.FC<AnyProps>;
  export const Rect: React.FC<AnyProps>;
  export const Path: React.FC<AnyProps>;
  export const G: React.FC<AnyProps>;
  export const Line: React.FC<AnyProps>;
  export const Polyline: React.FC<AnyProps>;
  export const Polygon: React.FC<AnyProps>;
  export const Defs: React.FC<AnyProps>;
  export const LinearGradient: React.FC<AnyProps>;
  export const Stop: React.FC<AnyProps>;
  export default Svg;
}

declare module "moti" {
  import * as React from "react";
  type AnyProps = Record<string, unknown> & {
    children?: React.ReactNode;
    style?: unknown;
    from?: Record<string, unknown>;
    animate?: Record<string, unknown>;
    transition?: Record<string, unknown>;
    exit?: Record<string, unknown>;
  };
  export const MotiView: React.FC<AnyProps>;
  export const MotiText: React.FC<AnyProps>;
  export const AnimatePresence: React.FC<{ children?: React.ReactNode }>;
}
