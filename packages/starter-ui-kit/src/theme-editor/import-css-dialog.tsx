// Paste-CSS import dialog. Accepts a `:root { … } .dark { … }` blob,
// runs it through `parseCssInput`, and merges the result into the
// editor store as a single checkpointed edit.

import { useState } from "react";

import { parseCssInput, useThemeEditorStore } from "@nube/starter-ui-core/theme-editor";

import { Button } from "../components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../components/ui/dialog";
import { Textarea } from "../components/ui/textarea";

export interface ImportCssDialogProps {
  /** Optional trigger; defaults to a "Import CSS" outline button. */
  children?: React.ReactNode;
}

export function ImportCssDialog({ children }: ImportCssDialogProps) {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleImport = () => {
    const parsed = parseCssInput(value);
    if (!parsed.light && !parsed.dark) {
      setError("No `:root { … }` or `.dark { … }` block found.");
      return;
    }
    const store = useThemeEditorStore.getState();
    store.checkpoint();
    const next = {
      light: { ...store.styles.light, ...(parsed.light ?? {}) },
      dark: { ...store.styles.dark, ...(parsed.dark ?? {}) },
    };
    store.applyPresetStyles(next);
    setValue("");
    setError(null);
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {children ?? (
          <Button type="button" variant="outline" size="sm">
            Import CSS
          </Button>
        )}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Import theme CSS</DialogTitle>
          <DialogDescription>
            Paste a CSS block containing <code>:root</code> and/or <code>.dark</code> rules. Recognised
            <code> --token</code> declarations will be merged into the editor.
          </DialogDescription>
        </DialogHeader>
        <Textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          rows={12}
          className="font-mono text-xs"
          placeholder=":root { --primary: oklch(0.55 0.22 257); }"
        />
        {error && <p className="text-sm text-destructive">{error}</p>}
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="ghost" type="button">
              Cancel
            </Button>
          </DialogClose>
          <Button type="button" onClick={handleImport}>
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
