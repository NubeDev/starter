// Semantic role → token key mapping.
//
// `Role` is the surface a component asks for ("I'm a danger button");
// the map resolves it to a concrete `(background, foreground, border)`
// triple of token keys from `palette.ts`. Web (`starter-ui-kit`) and
// mobile (`starter-ui-kit-native`) both consume this so a renderer
// asking for `Role.Danger` produces visually identical chrome on
// either runtime.

import type { ThemeStyleKey } from "./palette.js";

export type Role =
  | "surface"
  | "muted"
  | "primary"
  | "secondary"
  | "accent"
  | "danger"
  | "card"
  | "popover"
  | "sidebar";

export interface RoleTokens {
  background: ThemeStyleKey;
  foreground: ThemeStyleKey;
  border?: ThemeStyleKey;
}

export const ROLE_TO_TOKENS: Readonly<Record<Role, RoleTokens>> = {
  surface: { background: "background", foreground: "foreground", border: "border" },
  muted: { background: "muted", foreground: "muted-foreground" },
  primary: { background: "primary", foreground: "primary-foreground" },
  secondary: { background: "secondary", foreground: "secondary-foreground" },
  accent: { background: "accent", foreground: "accent-foreground" },
  danger: { background: "destructive", foreground: "destructive-foreground" },
  card: { background: "card", foreground: "card-foreground", border: "border" },
  popover: { background: "popover", foreground: "popover-foreground", border: "border" },
  sidebar: {
    background: "sidebar",
    foreground: "sidebar-foreground",
    border: "sidebar-border",
  },
};
