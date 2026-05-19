// Notes app. Sits on top of starter's React glue:
//   - `<AuthProvider>` + `useAuth()` from @nube/starter-ui-core
//   - `tokenStrategy` because we use auth-token (single bearer)
//   - `StarterClient` extended with `listNotes` / `createNote`
//
// Nothing in @nube/starter-ui-core or @nube/starter-client-ts was
// modified — both are consumed as published libraries.

import { useEffect, useState } from "react";
import { useAuth, AuthProvider, tokenStrategy } from "@nube/starter-ui-core/auth";
import { StarterClient } from "@nube/starter-client-ts";

import { NotesClient, type Note } from "./notes-client.js";

const client = new StarterClient({ baseUrl: "" });
const notesClient = new NotesClient(client);
const strategy = tokenStrategy();

export function App() {
  return (
    <AuthProvider client={client} strategy={strategy}>
      <Shell />
    </AuthProvider>
  );
}

function Shell() {
  const auth = useAuth();

  if (auth.status === "loading") return <p>checking session…</p>;
  if (auth.status === "unauthenticated") return <LoginForm />;
  return <NotesView />;
}

function LoginForm() {
  const auth = useAuth();
  const [token, setToken] = useState("");
  const [err, setErr] = useState<string | null>(null);

  return (
    <main style={{ maxWidth: 480, margin: "4rem auto", fontFamily: "sans-serif" }}>
      <h1>notes — sign in</h1>
      <p style={{ color: "#666" }}>
        Paste the bearer token printed by <code>notes claim --yes</code>.
      </p>
      <form
        onSubmit={async (e) => {
          e.preventDefault();
          setErr(null);
          try {
            await auth.login({ kind: "token", token });
          } catch (e) {
            setErr((e as Error).message);
          }
        }}
      >
        <input
          type="password"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          style={{ width: "100%", padding: 8, fontFamily: "monospace" }}
          placeholder="bearer token"
        />
        <button type="submit" style={{ marginTop: 12 }}>sign in</button>
      </form>
      {err && <p style={{ color: "crimson" }}>{err}</p>}
    </main>
  );
}

function NotesView() {
  const auth = useAuth();
  const [notes, setNotes] = useState<Note[]>([]);
  const [draft, setDraft] = useState("");
  const [err, setErr] = useState<string | null>(null);

  async function refresh() {
    try {
      setNotes(await notesClient.list());
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  return (
    <main style={{ maxWidth: 640, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <h1>notes</h1>
        <small>
          signed in as {auth.user?.subject} ·{" "}
          <button type="button" onClick={() => void auth.logout()}>sign out</button>
        </small>
      </header>

      <form
        onSubmit={async (e) => {
          e.preventDefault();
          if (!draft.trim()) return;
          try {
            await notesClient.create(draft);
            setDraft("");
            await refresh();
          } catch (e) {
            setErr((e as Error).message);
          }
        }}
      >
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="new note…"
          style={{ width: "100%", padding: 8 }}
        />
      </form>

      {err && <p style={{ color: "crimson" }}>{err}</p>}

      <ul style={{ listStyle: "none", padding: 0, marginTop: 16 }}>
        {notes.map((n) => (
          <li key={n.id} style={{ padding: "8px 0", borderBottom: "1px solid #eee" }}>
            <div>{n.body}</div>
            <small style={{ color: "#888" }}>{new Date(n.created_at).toLocaleString()}</small>
          </li>
        ))}
      </ul>
    </main>
  );
}
