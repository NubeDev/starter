// Concrete hex values for each `SduiAccent` role, mirrored from the
// Flutter `SduiTheme.light`/`dark` defaults in
// `rubix/flutter/packages/rubix_sdui/lib/src/widgets/sdui_theme.dart`
// and the React CSS-var contract in
// `rubix/docs/design/sdui/visual-design-spec.md`.
//
// Why hardcoded? The RN kit (`@nube/starter-ui-kit-native`) does not
// yet expose rubix-specific accent tokens — only abstract roles
// (`background`, `foreground`, `border`). Until the kit grows an
// `accents.leaf` token, the SDUI native renderer ships a local
// palette tuned per mode so visuals stay consistent with the web and
// Flutter renderers.

import type { SduiAccent } from "@nube/starter-ui-sdui-react/headless";

const LIGHT: Record<SduiAccent, string> = {
  leaf: "#4497A2", // teal500
  aqua: "#0EA5B7",
  sun:  "#F5A314", // yellow500
  sky:  "#3C83F6", // blue500
  warn: "#F59F0A", // amber500
};

const DARK: Record<SduiAccent, string> = {
  leaf: "#61A3AE", // teal400 lifted for dark bg
  aqua: "#67E8F9",
  sun:  "#FBBD41", // yellow400
  sky:  "#7DD3FC",
  warn: "#FDE68A",
};

export function accentHex(accent: SduiAccent, mode: "light" | "dark"): string {
  return (mode === "dark" ? DARK : LIGHT)[accent];
}

export const STATUS = {
  ok:     { light: "#21C45D", dark: "#22C55E" },
  danger: { light: "#EF4343", dark: "#FB7185" },
} as const;

export function trendColor(
  trend: string | undefined,
  mode: "light" | "dark",
): string | undefined {
  if (!trend) return undefined;
  if (trend.startsWith("+") || /^up\b/i.test(trend)) return STATUS.ok[mode];
  if (trend.startsWith("-") || /^down\b/i.test(trend)) return STATUS.danger[mode];
  return undefined;
}
