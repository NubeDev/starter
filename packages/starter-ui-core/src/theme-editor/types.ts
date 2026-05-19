// Theme-editor type surface. Mirrors the 38 shadcn CSS custom
// properties that `@nube/starter-ui-kit/styles.css` already declares on
// `:root`, plus the non-colour tokens (radius, fonts, shadow, spacing).
// Restyling these restyles every primitive in the kit.
//
// The shape is the public contract between the editor, the
// `ThemeTransport`, and any consumer-side persistence — keep it
// JSON-clean (string keys, string values) so it round-trips through
// `JSON.stringify` and a REST PUT without adapters.

/** Every editable token key. Order is irrelevant to the model but the
 * editor groups them (Brand / Surface / Text / Status / Sidebar /
 * Charts / Typography / Shape) for the UI. */
export type ThemeStyleKey =
  // Colour — paired light/dark, but the key itself is mode-agnostic.
  | "background"
  | "foreground"
  | "card"
  | "card-foreground"
  | "popover"
  | "popover-foreground"
  | "primary"
  | "primary-foreground"
  | "secondary"
  | "secondary-foreground"
  | "muted"
  | "muted-foreground"
  | "accent"
  | "accent-foreground"
  | "destructive"
  | "destructive-foreground"
  | "border"
  | "input"
  | "ring"
  | "chart-1"
  | "chart-2"
  | "chart-3"
  | "chart-4"
  | "chart-5"
  | "sidebar"
  | "sidebar-foreground"
  | "sidebar-primary"
  | "sidebar-primary-foreground"
  | "sidebar-accent"
  | "sidebar-accent-foreground"
  | "sidebar-border"
  | "sidebar-ring"
  // Shape.
  | "radius"
  // Typography.
  | "font-sans"
  | "font-serif"
  | "font-mono"
  | "letter-spacing"
  // Shadow.
  | "shadow-color"
  | "shadow-opacity"
  | "shadow-blur"
  | "shadow-spread"
  | "shadow-offset-x"
  | "shadow-offset-y";

/** A complete (or partial) token map for one mode. Values are stored as
 * the *author-typed* string (hex, oklch(), rem, font-family stack, …);
 * normalisation to OKLCH at apply-time lives in `utils/apply-theme.ts`
 * so the editor preserves what the user typed for round-tripping. */
export type ThemeStyleProps = Partial<Record<ThemeStyleKey, string>>;

/** Light + dark variant pair. Either map may be empty, in which case
 * the host stylesheet defaults from `globals.css` are used. */
export interface ThemeStyles {
  light: ThemeStyleProps;
  dark: ThemeStyleProps;
}

/** A named, gallery-displayable theme. */
export interface ThemePreset {
  /** Stable identifier; also used as the React key. */
  id: string;
  /** Display name shown on the gallery card. */
  label: string;
  /** Optional one-line description for tooltips / accessibility. */
  description?: string;
  /** Token values for both modes. */
  styles: ThemeStyles;
}

/** Shell-level branding that lives next to the token map. These
 * concerns (nav title, hidden features, logo/favicon) are
 * intentionally generic — a consumer wires them into their own app
 * shell however they like; starter's components do not assume a
 * specific layout beyond the live-preview replica. */
export interface ShellConfig {
  /** Title shown in the nav header. Empty string falls back to the
   * consumer's own default in their app shell. */
  nav_title: string;
  /** Feature flags the admin can hide from end-users. The strings are
   * arbitrary by design — starter does not enumerate which features a
   * consumer exposes; the editor just edits the list. */
  hide_features: string[];
}

/** The serialised payload that crosses the wire between editor and
 * backend. This is also the shape `ThemeTransport.load` returns and
 * `ThemeTransport.save` accepts. */
export interface ThemeDocument {
  theme_styles: ThemeStyles;
  shell: ShellConfig;
  /** Server-assigned URL of the currently-stored logo, if any. */
  logo_url?: string | null;
  /** Server-assigned URL of the currently-stored favicon, if any. */
  favicon_url?: string | null;
}

/** Which colour mode the editor is currently rendering / editing. */
export type ThemeMode = "light" | "dark";
