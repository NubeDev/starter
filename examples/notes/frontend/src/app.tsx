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
import { ExtensionsClient } from "./extensions-client.js";
import { ExtensionsView } from "./extensions-view.js";

const client = new StarterClient({ baseUrl: "" });
const notesClient = new NotesClient(client);
const extensionsClient = new ExtensionsClient(client);
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
  const [tab, setTab] = useState<"notes" | "extensions">("notes");
  const [adminAvailable, setAdminAvailable] = useState(false);
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
    // Probe the admin slice once. A 200 means the bearer carries
    // Role::Admin — show the tab. 403 / 401 / network errors hide it.
    void (async () => {
      try {
        await extensionsClient.list();
        setAdminAvailable(true);
      } catch {
        setAdminAvailable(false);
      }
    })();
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

      {adminAvailable && (
        <nav style={{ display: "flex", gap: 8, marginBottom: 16, borderBottom: "1px solid #eee" }}>
          <TabButton active={tab === "notes"} onClick={() => setTab("notes")}>notes</TabButton>
          <TabButton active={tab === "extensions"} onClick={() => setTab("extensions")}>
            extensions
          </TabButton>
        </nav>
      )}

      {tab === "extensions" ? (
        <ExtensionsView client={extensionsClient} />
      ) : (
        <NotesPanel
          notes={notes}
          draft={draft}
          setDraft={setDraft}
          err={err}
          setErr={setErr}
          refresh={refresh}
        />
      )}
    </main>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        background: "transparent",
        border: "none",
        borderBottom: active ? "2px solid #333" : "2px solid transparent",
        padding: "8px 4px",
        cursor: "pointer",
        fontWeight: active ? 600 : 400,
      }}
    >
      {children}
    </button>
  );
}

function NotesPanel({
  notes,
  draft,
  setDraft,
  err,
  setErr,
  refresh,
}: {
  notes: Note[];
  draft: string;
  setDraft: (s: string) => void;
  err: string | null;
  setErr: (s: string | null) => void;
  refresh: () => Promise<void>;
}) {
  return (
    <>
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
    </>
  );
}
