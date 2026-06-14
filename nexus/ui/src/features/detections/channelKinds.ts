// The notification-channel kinds the backend supports and their config fields.
// Drives the channel create form: a kind picker plus a schema-driven set of
// inputs per kind, replacing the old free-text kind + raw-JSON config. Each
// field maps one-to-one to a key in the channel's `config` object the API takes.

export type ChannelFieldType = "text" | "password" | "number" | "checkbox";

export interface ChannelField {
  key: string;
  label: string;
  type: ChannelFieldType;
  placeholder?: string;
  required?: boolean;
  /** A field whose value is a secret — never echoed back by the API on read. */
  secret?: boolean;
}

export interface ChannelKindSpec {
  kind: string;
  label: string;
  /** Hint shown under the kind picker. */
  description: string;
  fields: ChannelField[];
}

export const CHANNEL_KINDS: ChannelKindSpec[] = [
  {
    kind: "webhook",
    label: "Webhook",
    description: "POST the finding as JSON to a URL.",
    fields: [
      {
        key: "url",
        label: "URL",
        type: "text",
        placeholder: "https://hooks.example.com/…",
        required: true,
      },
    ],
  },
  {
    kind: "slack",
    label: "Slack",
    description: "Post to a Slack incoming-webhook URL.",
    fields: [
      {
        key: "url",
        label: "Incoming webhook URL",
        type: "password",
        placeholder: "https://hooks.slack.com/services/…",
        required: true,
        secret: true,
      },
    ],
  },
  {
    kind: "email",
    label: "Email (SMTP)",
    description: "Send mail through an SMTP server.",
    fields: [
      { key: "host", label: "SMTP host", type: "text", placeholder: "smtp.example.com", required: true },
      { key: "port", label: "Port", type: "number", placeholder: "587" },
      { key: "from", label: "From", type: "text", placeholder: "alerts@example.com", required: true },
      { key: "to", label: "To", type: "text", placeholder: "ops@example.com", required: true },
      { key: "username", label: "Username", type: "text" },
      { key: "password", label: "Password", type: "password", secret: true },
      { key: "starttls", label: "Use STARTTLS", type: "checkbox" },
    ],
  },
];

export function channelKind(kind: string): ChannelKindSpec | undefined {
  return CHANNEL_KINDS.find((k) => k.kind === kind);
}

/** Build the API `config` object from a kind's form values, coercing types and
 * dropping empty optional fields so the stored config stays minimal. */
export function buildChannelConfig(
  kind: string,
  values: Record<string, string | boolean>,
): Record<string, unknown> {
  const spec = channelKind(kind);
  if (!spec) return {};
  const config: Record<string, unknown> = {};
  for (const field of spec.fields) {
    const raw = values[field.key];
    if (field.type === "checkbox") {
      if (raw === true) config[field.key] = true;
      continue;
    }
    const str = typeof raw === "string" ? raw.trim() : "";
    if (str === "") continue;
    config[field.key] = field.type === "number" ? Number(str) : str;
  }
  return config;
}
