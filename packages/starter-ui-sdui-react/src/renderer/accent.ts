// Accent rotation for KPI tiles and chart series. Maps a logical
// accent name to the rubix theme tokens (var(--color-*)) so SDUI
// visuals follow the active palette (nube / ocean / sunset) without
// renderer changes.
//
// Server-side IR can pin a tile to a specific accent via `node.accent`
// or `intent`; otherwise we derive one from `node.id` so siblings get
// stable, visually-distinct colors across re-renders.

export const SDUI_ACCENTS = ["leaf", "aqua", "sun", "sky", "warn"] as const;
export type SduiAccent = (typeof SDUI_ACCENTS)[number];

const ACCENT_VAR: Record<SduiAccent, string> = {
  leaf: "var(--color-leaf)",
  aqua: "var(--color-aqua)",
  sun: "var(--color-sun)",
  sky: "var(--color-sky)",
  warn: "var(--color-warn)",
};

const INTENT_MAP: Record<string, SduiAccent> = {
  primary: "leaf",
  positive: "leaf",
  good: "leaf",
  info: "sky",
  warn: "warn",
  warning: "warn",
  energy: "sun",
  cool: "aqua",
};

function hash(input: string): number {
  let h = 0;
  for (let i = 0; i < input.length; i++) h = (h * 31 + input.charCodeAt(i)) | 0;
  return Math.abs(h);
}

export function resolveAccent(node: {
  id?: unknown;
  accent?: unknown;
  intent?: unknown;
}): SduiAccent {
  if (typeof node.accent === "string" && (SDUI_ACCENTS as readonly string[]).includes(node.accent)) {
    return node.accent as SduiAccent;
  }
  if (typeof node.intent === "string" && INTENT_MAP[node.intent]) {
    return INTENT_MAP[node.intent]!;
  }
  const seed = typeof node.id === "string" ? node.id : "";
  // Skip `warn` in the auto-rotation — reserve it for explicit intent.
  const palette = SDUI_ACCENTS.slice(0, 4);
  return palette[hash(seed) % palette.length]!;
}

export function accentVar(accent: SduiAccent): string {
  return ACCENT_VAR[accent];
}

export function accentByIndex(index: number): SduiAccent {
  const palette = SDUI_ACCENTS.slice(0, 4);
  return palette[index % palette.length]!;
}
