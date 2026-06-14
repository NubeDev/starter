// `templates-tab.tsx` — list templates; edit YAML in a textarea; validate+save.
import * as React from "react";
import { Check, FileCode2, FilePlus2, QrCode } from "lucide-react";
import { getTemplateYaml, listTemplates, templateUpsert } from "../bc-api";
import { useRefreshKey } from "../refresh";
import { TemplateQrDialog } from "./template-qr-dialog";
import type { TemplateRow } from "../bc-types";

export function TemplatesTab(): React.ReactElement {
  const [templates, setTemplates] = React.useState<ReadonlyArray<TemplateRow>>([]);
  const [selected, setSelected] = React.useState<string | null>(null);
  const [yaml, setYaml] = React.useState("");
  const [listError, setListError] = React.useState<string | null>(null);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [qrFor, setQrFor] = React.useState<TemplateRow | null>(null);
  const refresh = useRefreshKey();

  const loadList = React.useCallback(() => {
    listTemplates()
      .then(setTemplates)
      .catch((e: unknown) => setListError(e instanceof Error ? e.message : String(e)));
  }, []);
  React.useEffect(loadList, [loadList, refresh]);

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

  const current = templates.find((t) => t.template === selected);

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-[280px_1fr]">
      <div className="ext-glass flex flex-col">
        <header className="flex items-center justify-between border-b border-border/60 px-3 py-2.5">
          <span className="ext-eyebrow">Inventory</span>
          <button type="button" onClick={startNew} className="flex cursor-pointer items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/10">
            <FilePlus2 className="size-3.5" /> New
          </button>
        </header>
        {listError ? (
          <p className="px-3 py-2 text-sm text-destructive">{listError}</p>
        ) : templates.length === 0 ? (
          <p className="px-3 py-3 text-sm italic text-muted-foreground">No templates yet.</p>
        ) : (
          <ul className="max-h-[60vh] overflow-y-auto p-1.5">
            {templates.map((t) => {
              const on = selected === t.template;
              return (
              <li
                key={t.template}
                className={
                  "group flex items-start gap-1 rounded-lg pr-1.5 transition-colors " +
                  (on ? "bg-primary/10 ring-1 ring-primary/30" : "hover:bg-accent")
                }
              >
                <button
                  type="button"
                  onClick={() => open(t.template)}
                  className="flex min-w-0 flex-1 cursor-pointer items-start gap-2.5 rounded-lg px-2.5 py-2 text-left"
                >
                  <span className={"mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md " + (on ? "bg-primary/15 text-primary" : "bg-muted/40 text-muted-foreground")}>
                    <FileCode2 className="size-4" />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className={"block truncate text-sm font-medium " + (on ? "text-foreground" : "text-foreground/85")}>{t.display_name}</span>
                    <span className="block truncate font-mono text-[11px] text-muted-foreground">
                      {t.template} · v{t.version}
                    </span>
                    {t.network ? (
                      <span className="mt-1 inline-flex rounded bg-muted/50 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-muted-foreground">
                        {t.network}
                      </span>
                    ) : null}
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => setQrFor(t)}
                  aria-label={`Make a QR sticker for ${t.display_name}`}
                  title="Make QR sticker"
                  className="mt-1.5 flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground opacity-0 transition-all hover:bg-primary/10 hover:text-primary group-hover:opacity-100"
                >
                  <QrCode className="size-4" />
                </button>
              </li>
              );
            })}
          </ul>
        )}
      </div>

      <div className="ext-glass flex flex-col overflow-hidden">
        {selected === null ? (
          <div className="flex h-[60vh] items-center justify-center px-6 text-center text-sm italic text-muted-foreground">
            Select a template, or create a new one.
          </div>
        ) : (
          <>
            <header className="flex items-center justify-between gap-2 border-b border-border/60 px-4 py-2.5">
              <div className="flex min-w-0 items-center gap-2">
                <FileCode2 className="size-4 shrink-0 text-muted-foreground" />
                <span className="truncate font-mono text-sm text-foreground">
                  {selected ? `${selected}.yaml` : "new-template.yaml"}
                </span>
                {current ? (
                  <span className="rounded border border-border/60 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                    v{current.version}
                  </span>
                ) : (
                  <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-amber-400">
                    Draft
                  </span>
                )}
              </div>
            </header>
            <textarea
              value={yaml}
              onChange={(e) => setYaml(e.target.value)}
              spellCheck={false}
              aria-label="Template YAML"
              className="h-[55vh] w-full resize-none border-0 bg-background/40 p-4 font-mono text-xs leading-relaxed text-foreground outline-none focus:ring-0"
            />
            {saveError ? (
              <div role="alert" className="mx-3 mb-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {saveError}
              </div>
            ) : null}
            <footer className="flex items-center justify-between gap-2 border-t border-border/60 px-4 py-2.5">
              <div className="flex items-center gap-2 text-xs">
                {saved ? (
                  <span className="inline-flex items-center gap-1.5 font-medium text-emerald-400">
                    <Check className="size-3.5" /> {saved}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1.5 text-muted-foreground">
                    <span className="inline-block size-1.5 rounded-full bg-amber-500" />
                    Draft mode · unsaved changes
                  </span>
                )}
              </div>
              <button
                type="button"
                onClick={save}
                disabled={busy}
                className="cursor-pointer rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {busy ? "Saving…" : "Validate & Save"}
              </button>
            </footer>
          </>
        )}
      </div>

      {qrFor ? <TemplateQrDialog template={qrFor} onClose={() => setQrFor(null)} /> : null}
    </div>
  );
}
