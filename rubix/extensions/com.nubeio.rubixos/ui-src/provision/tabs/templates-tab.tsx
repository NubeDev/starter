// `templates-tab.tsx` — list templates; edit YAML in a textarea; validate+save.
import * as React from "react";
import { FilePlus2 } from "lucide-react";
import { getTemplateYaml, listTemplates, templateUpsert } from "../bc-api";
import type { TemplateRow } from "../bc-types";

export function TemplatesTab(): React.ReactElement {
  const [templates, setTemplates] = React.useState<ReadonlyArray<TemplateRow>>([]);
  const [selected, setSelected] = React.useState<string | null>(null);
  const [yaml, setYaml] = React.useState("");
  const [listError, setListError] = React.useState<string | null>(null);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);

  const loadList = React.useCallback(() => {
    listTemplates()
      .then(setTemplates)
      .catch((e: unknown) => setListError(e instanceof Error ? e.message : String(e)));
  }, []);
  React.useEffect(loadList, [loadList]);

  const open = (template: string) => {
    setSelected(template);
    setSaved(null);
    setSaveError(null);
    setYaml("# loading…");
    getTemplateYaml(template)
      .then((rs) => setYaml(rs[0]?.yaml ?? ""))
      .catch((e: unknown) => setSaveError(e instanceof Error ? e.message : String(e)));
  };

  const startNew = () => {
    setSelected("");
    setSaved(null);
    setSaveError(null);
    setYaml("template: my_device\nversion: 1\ndisplay_name: My Device\nnetwork: bacnet\ncategory: sensor\npoints: []\n");
  };

  const save = () => {
    setBusy(true);
    setSaveError(null);
    setSaved(null);
    templateUpsert(yaml)
      .then((r) => {
        setSaved(`Saved (${r.operation}, ${r.affected} affected)`);
        loadList();
      })
      .catch((e: unknown) => setSaveError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-[260px_1fr]">
      <div className="rounded-lg border border-border/60 bg-card">
        <header className="flex items-center justify-between border-b border-border/60 px-3 py-2">
          <span className="text-sm font-medium">Templates</span>
          <button type="button" onClick={startNew} className="flex items-center gap-1 text-xs text-primary hover:underline">
            <FilePlus2 className="size-3.5" /> New
          </button>
        </header>
        {listError ? (
          <p className="px-3 py-2 text-sm text-destructive">{listError}</p>
        ) : templates.length === 0 ? (
          <p className="px-3 py-2 text-sm italic text-muted-foreground">No templates yet.</p>
        ) : (
          <ul className="max-h-[60vh] overflow-y-auto p-1">
            {templates.map((t) => (
              <li key={t.template}>
                <button
                  type="button"
                  onClick={() => open(t.template)}
                  className={
                    "w-full rounded px-2 py-1.5 text-left text-sm hover:bg-accent " +
                    (selected === t.template ? "bg-accent text-accent-foreground" : "text-foreground/85")
                  }
                >
                  <div className="truncate">{t.display_name}</div>
                  <div className="truncate text-xs text-muted-foreground">
                    {t.template} · v{t.version}
                  </div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="flex flex-col gap-2">
        {selected === null ? (
          <p className="text-sm italic text-muted-foreground">Select a template, or create a new one.</p>
        ) : (
          <>
            <textarea
              value={yaml}
              onChange={(e) => setYaml(e.target.value)}
              spellCheck={false}
              aria-label="Template YAML"
              className="h-[60vh] w-full resize-none rounded-md border border-border/60 bg-background p-3 font-mono text-xs text-foreground outline-none focus:border-primary"
            />
            {saveError ? (
              <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {saveError}
              </div>
            ) : null}
            {saved ? <div className="text-sm text-emerald-500">{saved}</div> : null}
            <button
              type="button"
              onClick={save}
              disabled={busy}
              className="self-start rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground disabled:opacity-50"
            >
              {busy ? "Saving…" : "Validate & save"}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
