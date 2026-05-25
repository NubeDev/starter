// connection/types.ts — shared row type. Kept in one file so the verbs
// don't redeclare it; if it grows it splits per LOCAL-DB.md's verb-per-
// file pattern.

/** A saved connection to a remote rubix-agent server. */
export interface Connection {
  /** ULID. */
  readonly id: string;
  /** Operator-chosen short label ("Home lab", "Site A"). */
  readonly label: string;
  /** Server URL, no trailing slash, with scheme. */
  readonly baseUrl: string;
  /** Optional hex colour for UI tagging. */
  readonly colour: string;
  /** Unix ms. */
  readonly createdAt: number;
  /** Unix ms, null until first probe. */
  readonly lastSeenAt: number | null;
  /** From `/healthz`. */
  readonly agentVersion: string | null;
  /** Operator-facing notes (markdown allowed; rendered as plain text v1). */
  readonly notes: string;
}

/** SQLite row shape (snake_case). */
export interface ConnectionRow {
  id: string;
  label: string;
  base_url: string;
  colour: string;
  created_at: number;
  last_seen_at: number | null;
  agent_version: string | null;
  notes: string;
}

export function rowToConnection(r: ConnectionRow): Connection {
  return {
    id: r.id,
    label: r.label,
    baseUrl: r.base_url,
    colour: r.colour,
    createdAt: r.created_at,
    lastSeenAt: r.last_seen_at,
    agentVersion: r.agent_version,
    notes: r.notes,
  };
}
