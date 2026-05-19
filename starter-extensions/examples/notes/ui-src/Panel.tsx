// The notes panel — a single React component the host mounts into its
// `sidebar` slot via Module Federation. Reads + writes go through
// `useHostClient()` (SCOPE R11: UI extensions never raw-`fetch`),
// against the same `POST /notes` / `GET /notes` routes the Rust side
// of this bundle contributes via `contributes.rest`.

import * as React from "react";

import {
  BlockShell,
  useHostClient,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";

interface Note {
  id: number;
  body: string;
}

export default function NotesPanel(): React.ReactElement {
  return (
    <BlockShell>
      <NotesPanelInner />
    </BlockShell>
  );
}

function NotesPanelInner(): React.ReactElement {
  const slot = useSlotContext();
  const client = useHostClient();
  const [notes, setNotes] = React.useState<Note[]>([]);
  const [draft, setDraft] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    try {
      const res = await client.get("/notes");
      const body = (await res.json()) as { notes: Note[] };
      setNotes(body.notes ?? []);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [client]);

  React.useEffect(() => {
    void refresh();
  }, [refresh]);

  const onAdd = async (event: React.FormEvent) => {
    event.preventDefault();
    const body = draft.trim();
    if (body.length === 0) return;
    try {
      await client.post("/notes", { body });
      setDraft("");
      await refresh();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <section className="notes-panel">
      <header>
        <h3>Notes — {slot.extensionId}</h3>
        <small>
          mounted in <code>{slot.slotId}</code> (theme: <em>{slot.theme}</em>)
        </small>
      </header>

      <form onSubmit={onAdd}>
        <input
          aria-label="new note"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="write a note…"
        />
        <button type="submit" disabled={draft.trim().length === 0}>
          add
        </button>
      </form>

      {error !== null && <p className="notes-panel-error">{error}</p>}

      <ul>
        {notes.map((n) => (
          <li key={n.id}>
            <strong>#{n.id}</strong> {n.body}
          </li>
        ))}
        {notes.length === 0 && error === null && (
          <li className="notes-panel-empty">no notes yet</li>
        )}
      </ul>
    </section>
  );
}
