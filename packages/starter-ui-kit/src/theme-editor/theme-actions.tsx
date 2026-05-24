// Toolbar — save, reset, undo/redo, mode toggle, export (CSS/JSON/YAML),
// and the import-CSS dialog trigger.
//
// The save callback is supplied by the host page (it owns the toast
// adapter and any post-save side-effects); this component is purely
// visual / wiring.

import {
  generateCssString,
  generateYamlString,
  useThemeEditorStore,
} from "@nube/starter-ui-core/theme-editor";

import { Button } from "../components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../components/ui/dropdown-menu";
import { ImportCssDialog } from "./import-css-dialog.js";

export interface ThemeActionsProps {
  /** Caller's save handler — typically `useThemeEditor().save`. */
  onSave: () => void | Promise<void>;
  /** Caller's discard handler — typically `useThemeEditor().reload`. */
  onDiscard: () => void | Promise<void>;
  /** Disabled state for the save button (e.g. while a request is in flight). */
  saving?: boolean;
}

export function ThemeActions({ onSave, onDiscard, saving }: ThemeActionsProps) {
  const isDirty = useThemeEditorStore((s) => s.isDirty);
  const mode = useThemeEditorStore((s) => s.mode);
  const setMode = useThemeEditorStore((s) => s.setMode);
  const undo = useThemeEditorStore((s) => s.undo);
  const redo = useThemeEditorStore((s) => s.redo);
  const reset = useThemeEditorStore((s) => s.reset);

  const copyCss = () => {
    const { styles } = useThemeEditorStore.getState();
    void navigator.clipboard.writeText(generateCssString(styles));
  };
  const copyJson = () => {
    const { styles, shell } = useThemeEditorStore.getState();
    void navigator.clipboard.writeText(JSON.stringify({ theme_styles: styles, shell }, null, 2));
  };
  const copyYaml = () => {
    const { styles } = useThemeEditorStore.getState();
    void navigator.clipboard.writeText(generateYamlString(styles));
  };

  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="flex rounded-md border border-border">
        <Button
          type="button"
          size="sm"
          variant={mode === "light" ? "default" : "ghost"}
          className="rounded-r-none"
          onClick={() => setMode("light")}
        >
          Light
        </Button>
        <Button
          type="button"
          size="sm"
          variant={mode === "dark" ? "default" : "ghost"}
          className="rounded-l-none"
          onClick={() => setMode("dark")}
        >
          Dark
        </Button>
      </div>

      <Button type="button" size="sm" variant="ghost" onClick={() => undo()}>
        Undo
      </Button>
      <Button type="button" size="sm" variant="ghost" onClick={() => redo()}>
        Redo
      </Button>

      <ImportCssDialog />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button type="button" size="sm" variant="outline">
            Export
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={copyCss}>Copy CSS</DropdownMenuItem>
          <DropdownMenuItem onSelect={copyJson}>Copy JSON</DropdownMenuItem>
          <DropdownMenuItem onSelect={copyYaml}>Copy YAML</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <div className="ml-auto flex gap-2">
        <Button type="button" size="sm" variant="ghost" onClick={() => void onDiscard()} disabled={!isDirty}>
          Discard
        </Button>
        <Button type="button" size="sm" variant="outline" onClick={() => reset()}>
          Reset
        </Button>
        <Button type="button" size="sm" onClick={() => void onSave()} disabled={!isDirty || saving}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}
