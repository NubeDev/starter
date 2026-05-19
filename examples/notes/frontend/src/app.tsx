// Notes app with shadcn/ui-style components and live extension rendering.

import { useEffect, useMemo, useState } from "react";
import { useAuth, AuthProvider, tokenStrategy } from "@nube/starter-ui-core/auth";
import {
  applyThemeToElement,
  httpThemeTransport,
  useThemeEditorStore,
} from "@nube/starter-ui-core/theme-editor";
import { ThemeEditorPage } from "@nube/starter-ui-kit/theme-editor";
import { StarterClient } from "@nube/starter-client-ts";
import { ExtensionHostProvider, ExtensionSlot } from "@nube/starter-ext-ui";

import { NotesClient, type Note } from "./notes-client.js";
import { ExtensionsClient } from "./extensions-client.js";
import { ExtensionsView } from "./extensions-view.js";
import { createExtensionHost, loadExtensionRemotes } from "./extension-host.js";

import {
  Button,
  Input,
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  Badge,
  Separator,
  Tabs,
  TabsList,
  TabsTrigger,
} from "./ui.js";

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

  if (auth.status === "loading") return <p style={{ padding: 32 }}>checking session…</p>;
  if (auth.status === "unauthenticated") return <LoginForm />;
  return <AuthenticatedApp />;
}

function LoginForm() {
  const auth = useAuth();
  const [token, setToken] = useState("");
  const [err, setErr] = useState<string | null>(null);

  return (
    <main style={{ maxWidth: 400, margin: "6rem auto", padding: "0 1rem" }}>
      <Card>
        <CardHeader>
          <CardTitle>notes — sign in</CardTitle>
          <p className="ui-card-desc">
            Paste the bearer token printed by <code>notes claim --yes</code>.
          </p>
        </CardHeader>
        <CardContent>
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
            style={{ display: "flex", flexDirection: "column", gap: 12 }}
          >
            <Input
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="bearer token"
            />
            <Button type="submit">Sign in</Button>
          </form>
          {err && <p style={{ color: "var(--destructive)", marginTop: 8, fontSize: "0.85rem" }}>{err}</p>}
        </CardContent>
      </Card>
    </main>
  );
}

function AuthenticatedApp() {
  const auth = useAuth();
  const [tab, setTab] = useState<"notes" | "extensions" | "theme">("notes");
  const [adminAvailable, setAdminAvailable] = useState(false);

  // One shared `ThemeTransport` for both the top-level hydration and
  // the `<ThemeEditorPage>` on the Theme tab.
  const themeTransport = useMemo(() => httpThemeTransport({ client }), []);

  // Live token map driven by the shared `useThemeEditorStore`. When
  // admin saves a new preset inside the editor, `markSaved()` keeps
  // the styles in the store, so this selector re-fires and the host
  // re-applies the tokens to `<html>` + every `<ExtensionSlot>`.
  const themeMode = useThemeEditorStore((s) => s.mode);
  const themeTokens = useThemeEditorStore((s) => s.styles[s.mode]);
  const hydrateTheme = useThemeEditorStore((s) => s.hydrate);

  // One-shot hydration on first authenticated render so the saved
  // theme applies before admin ever opens the Theme tab. Errors are
  // swallowed — the editor tab will surface its own error state and
  // the CSS-default oklch tokens from `globals.css` keep the UI
  // looking sensible meanwhile.
  useEffect(() => {
    void (async () => {
      try {
        const doc = await themeTransport.load();
        hydrateTheme(doc.theme_styles, doc.shell);
      } catch {
        /* keep CSS defaults */
      }
    })();
  }, [themeTransport, hydrateTheme]);

  // Create and bootstrap the extension host
  const host = useMemo(() => createExtensionHost({ client }), []);
  const [hostReady, setHostReady] = useState(false);

  useEffect(() => {
    void loadExtensionRemotes(host).then(() => setHostReady(true));
  }, [host]);

  // Apply the live token map to the document root so the host UI
  // (notes, extensions tabs) reflects every theme change immediately.
  useEffect(() => {
    if (Object.keys(themeTokens).length > 0) {
      applyThemeToElement(document.documentElement, themeTokens, themeMode);
    }
  }, [themeTokens, themeMode]);

  useEffect(() => {
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
    <ExtensionHostProvider host={host}>
      <main style={{ maxWidth: 720, margin: "2rem auto", padding: "0 1rem" }}>
        {/* Header */}
        <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
          <h1 style={{ fontSize: "1.5rem", fontWeight: 700 }}>notes</h1>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Badge variant={hostReady ? "success" : "secondary"}>
              {hostReady ? "● extensions ready" : "○ loading…"}
            </Badge>
            <Badge variant="secondary">{auth.user?.subject?.slice(0, 8)}…</Badge>
            <Button variant="ghost" size="sm" onClick={() => void auth.logout()}>
              Sign out
            </Button>
          </div>
        </header>

        <Separator />

        {/* Tabs */}
        {adminAvailable && (
          <div style={{ marginTop: 16 }}>
            <Tabs>
              <TabsList>
                <TabsTrigger active={tab === "notes"} onClick={() => setTab("notes")}>
                  Notes
                </TabsTrigger>
                <TabsTrigger active={tab === "extensions"} onClick={() => setTab("extensions")}>
                  Extensions
                </TabsTrigger>
                <TabsTrigger active={tab === "theme"} onClick={() => setTab("theme")}>
                  Theme
                </TabsTrigger>
              </TabsList>
            </Tabs>
          </div>
        )}

        {/* Tab Content */}
        <div style={{ marginTop: 16 }}>
          {tab === "extensions" ? (
            <ExtensionsView client={extensionsClient} />
          ) : tab === "theme" ? (
            <ThemeEditorPage transport={themeTransport} />
          ) : (
            <NotesPanel />
          )}
        </div>

        {/* Extension sidebar slot — renders live extension UIs.
            `themeTokens` threads the active host preset into
            `SlotContext` so the extension's `useHostTheme()` sees
            the live token map (charts, canvas, etc.). */}
        {hostReady && (
          <div style={{ marginTop: 24 }}>
            <ExtensionSlot
              id="sidebar"
              theme={themeMode}
              themeTokens={themeTokens}
            />
          </div>
        )}
      </main>
    </ExtensionHostProvider>
  );
}

function NotesPanel() {
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

  useEffect(() => { void refresh(); }, []);

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
        style={{ display: "flex", gap: 8 }}
      >
        <Input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Write a new note…"
          style={{ flex: 1 }}
        />
        <Button type="submit">Add</Button>
      </form>

      {err && <p style={{ color: "var(--destructive)", marginTop: 8, fontSize: "0.85rem" }}>{err}</p>}

      <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 8 }}>
        {notes.map((n) => (
          <Card key={n.id}>
            <CardContent style={{ padding: "12px 16px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                <span>{n.body}</span>
                <small style={{ color: "var(--muted-foreground)", fontSize: "0.75rem" }}>
                  {new Date(n.created_at).toLocaleString()}
                </small>
              </div>
            </CardContent>
          </Card>
        ))}
        {notes.length === 0 && (
          <p style={{ color: "var(--muted-foreground)", textAlign: "center", padding: 32 }}>
            No notes yet. Write one above!
          </p>
        )}
      </div>
    </>
  );
}
