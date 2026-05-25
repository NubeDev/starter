// Ambient shims for the native peer deps.
//
// `@nube/starter-ui-kit-native` declares `react-native`,
// `react-native-svg`, `react-native-reanimated`, and `moti` as peer
// dependencies — the consuming Expo / RN app installs them. To keep
// the workspace typecheck (`tsc --noEmit`) and the vitest runs free
// of the heavy native toolchains, we declare loose-but-honest module
// shapes here. The vitest config aliases each module to a host-element
// mock under `src/__mocks__/`, which is the source of truth for
// behaviour. These shims are types-only.

declare module "react-native" {
  import * as React from "react";

  // Loose accessibility surface — narrowed in JSDoc on each primitive.
  export interface AccessibilityProps {
    accessible?: boolean;
    accessibilityLabel?: string;
    accessibilityHint?: string;
    accessibilityRole?: string;
    accessibilityState?: Record<string, boolean | undefined>;
    accessibilityValue?: {
      min?: number;
      max?: number;
      now?: number;
      text?: string;
    };
    testID?: string;
  }

  export type StyleProp<T = unknown> = T | T[] | null | undefined;
  export type ViewStyle = Record<string, unknown>;
  export type TextStyle = Record<string, unknown>;

  export interface ViewProps extends AccessibilityProps {
    style?: StyleProp<ViewStyle>;
    children?: React.ReactNode;
    onLayout?: (event: unknown) => void;
    pointerEvents?: "auto" | "none" | "box-none" | "box-only";
  }
  export interface TextProps extends AccessibilityProps {
    style?: StyleProp<TextStyle>;
    children?: React.ReactNode;
    numberOfLines?: number;
  }
  export interface PressableProps extends AccessibilityProps {
    onPress?: (event?: unknown) => void;
    onLongPress?: (event?: unknown) => void;
    disabled?: boolean;
    style?: StyleProp<ViewStyle>;
    children?: React.ReactNode;
    hitSlop?: number | { top?: number; bottom?: number; left?: number; right?: number };
  }
  export interface TextInputProps extends AccessibilityProps {
    value?: string;
    defaultValue?: string;
    onChangeText?: (text: string) => void;
    placeholder?: string;
    placeholderTextColor?: string;
    secureTextEntry?: boolean;
    editable?: boolean;
    keyboardType?: string;
    autoCapitalize?: "none" | "sentences" | "words" | "characters";
    autoCorrect?: boolean;
    style?: StyleProp<TextStyle>;
    onFocus?: () => void;
    onBlur?: () => void;
  }
  export interface ModalProps {
    visible?: boolean;
    transparent?: boolean;
    animationType?: "none" | "slide" | "fade";
    onRequestClose?: () => void;
    children?: React.ReactNode;
  }
  export interface ScrollViewProps extends ViewProps {
    horizontal?: boolean;
    showsHorizontalScrollIndicator?: boolean;
    showsVerticalScrollIndicator?: boolean;
    contentContainerStyle?: StyleProp<ViewStyle>;
  }
  export interface ActivityIndicatorProps extends AccessibilityProps {
    size?: "small" | "large" | number;
    color?: string;
    animating?: boolean;
    style?: StyleProp<ViewStyle>;
  }

  export const View: React.ForwardRefExoticComponent<
    ViewProps & React.RefAttributes<unknown>
  >;
  export const Text: React.ForwardRefExoticComponent<
    TextProps & React.RefAttributes<unknown>
  >;
  export const Pressable: React.ForwardRefExoticComponent<
    PressableProps & React.RefAttributes<unknown>
  >;
  export const TextInput: React.ForwardRefExoticComponent<
    TextInputProps & React.RefAttributes<unknown>
  >;
  export const Modal: React.FC<ModalProps>;
  export const ScrollView: React.ForwardRefExoticComponent<
    ScrollViewProps & React.RefAttributes<unknown>
  >;
  export const ActivityIndicator: React.FC<ActivityIndicatorProps>;

  export const StyleSheet: {
    create<T extends Record<string, ViewStyle | TextStyle>>(s: T): T;
    flatten(
      s: StyleProp<ViewStyle | TextStyle>,
    ): ViewStyle | TextStyle;
    absoluteFillObject: ViewStyle;
    hairlineWidth: number;
  };

  export const Platform: {
    OS: "ios" | "android" | "web" | "windows" | "macos";
    select<T>(opts: { ios?: T; android?: T; web?: T; default?: T }): T | undefined;
  };

  export const Dimensions: {
    get(name: "window" | "screen"): {
      width: number;
      height: number;
      scale: number;
      fontScale: number;
    };
  };

  export function useColorScheme(): "light" | "dark" | null | undefined;

  export interface PanResponderGestureState {
    dx: number;
    dy: number;
    moveX: number;
    moveY: number;
    x0: number;
    y0: number;
  }
  export const PanResponder: {
    create(handlers: {
      onStartShouldSetPanResponder?: () => boolean;
      onMoveShouldSetPanResponder?: () => boolean;
      onPanResponderMove?: (e: unknown, g: PanResponderGestureState) => void;
      onPanResponderRelease?: (e: unknown, g: PanResponderGestureState) => void;
    }): { panHandlers: Record<string, unknown> };
  };
}

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
  export const Defs: React.FC<AnyProps>;
  export const LinearGradient: React.FC<AnyProps>;
  export const Stop: React.FC<AnyProps>;
  export default Svg;
}

declare module "react-native-reanimated" {
  export const Easing: {
    bezier(x1: number, y1: number, x2: number, y2: number): (t: number) => number;
    linear: (t: number) => number;
    out(easing: (t: number) => number): (t: number) => number;
  };
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
