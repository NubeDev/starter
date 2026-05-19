// Consumer-owned client. Composes `StarterClient` rather than
// patching it via declaration-merging — the wrapper pattern works
// cleanly from outside the package and keeps the surface explicit.
// Starter's auth/me/login methods stay accessible via `.starter`.

import { StarterClient, StarterError } from "@nube/starter-client-ts";

export interface Note {
  id: string;
  body: string;
  created_at: string;
  created_by: string;
}

export class NotesClient {
  constructor(public readonly starter: StarterClient) {}

  async list(): Promise<Note[]> {
    const res = await this.starter.fetch(`${this.starter.baseUrl}/notes`, {
      headers: this.starter.headers,
    });
    if (!res.ok) throw await StarterError.fromResponse(res);
    return (await res.json()) as Note[];
  }

  async create(body: string): Promise<Note> {
    const res = await this.starter.fetch(`${this.starter.baseUrl}/notes`, {
      method: "POST",
      headers: { ...this.starter.headers, "content-type": "application/json" },
      body: JSON.stringify({ body }),
    });
    if (!res.ok) throw await StarterError.fromResponse(res);
    return (await res.json()) as Note;
  }
}
