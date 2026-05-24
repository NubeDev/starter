// Top-level page. Composes the gallery, colour editor, branding panel,
// live preview, and toolbar. Wires up keyboard shortcuts and reads
// the `useThemeEditor` lifecycle hook against a consumer-supplied
// `ThemeTransport`.
//
// Layout: two-column split, gallery+editor on the left, preview on
// the right. Stacks vertically on small viewports.

import { useEffect, useMemo } from "react";

import {
  useThemeEditor,
  useThemePresets,
  useThemeEditorStore,
} from "@nube/starter-ui-core/theme-editor";
import type { ThemeTransport } from "@nube/starter-ui-core/theme-editor";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs";
import { cn } from "../lib/utils";

import { BrandingEditor } from "./branding-editor.js";
import { ColorTokenEditor } from "./color-token-editor.js";
import { LivePreview } from "./live-preview.js";
import { ThemeActions } from "./theme-actions.js";
import { ThemeGallery } from "./theme-gallery.js";

export interface ThemeEditorPageProps {
  /** Persistence seam. Default consumers pass `httpThemeTransport({ client })`. */
  transport: ThemeTransport;
  /** Optional feature flags surfaced in the Branding panel. */
  hideFeatureOptions?: ReadonlyArray<{ id: string; label: string }>;
  /** Toast helper — receives one of `"saved"`, `"save_failed"`,
   * `"copied"`. Defaults to a no-op so the editor works in a tree
   * without a toaster. */
  onNotify?: (kind: "saved" | "save_failed" | "copied", message?: string) => void;
  className?: string;
}

export function ThemeEditorPage({
  transport,
  hideFeatureOptions,
  onNotify = () => {},
  className,
}: ThemeEditorPageProps) {
  const presets = useThemePresets();
  const { isLoading, error, logoUrl, faviconUrl, save, reload } = useThemeEditor(transport);
  const isDirty = useThemeEditorStore((s) => s.isDirty);

  // Keyboard shortcuts: Ctrl/Cmd+S = save, Ctrl/Cmd+Z = undo,
  // Ctrl/Cmd+Shift+Z (or Ctrl+Y) = redo. Bound on the window so they
  // work anywhere on the page.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      const key = e.key.toLowerCase();
      if (key === "s") {
        e.preventDefault();
        void handleSave();
      } else if (key === "z" && !e.shiftKey) {
        e.preventDefault();
        useThemeEditorStore.getState().undo();
      } else if ((key === "z" && e.shiftKey) || key === "y") {
        e.preventDefault();
        useThemeEditorStore.getState().redo();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // `handleSave` is referenced via closure; defined below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSave = useMemo(
    () => async () => {
      try {
        await save();
        onNotify("saved", "Theme saved");
      } catch (e) {
        onNotify("save_failed", e instanceof Error ? e.message : String(e));
      }
    },
    [save, onNotify],
  );

  return (
    <div className={cn("flex h-full flex-col gap-4 p-4", className)}>
      <header className="flex items-center justify-between">
        <div className="flex items-baseline gap-2">
          <h1 className="text-2xl font-semibold">Theme</h1>
          {isDirty && (
            <span
              className="size-2 rounded-full bg-amber-500"
              title="Unsaved changes"
              aria-label="Unsaved changes"
            />
          )}
        </div>
        <ThemeActions onSave={handleSave} onDiscard={reload} saving={isLoading} />
      </header>

      {error && (
        <div
          role="alert"
          className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
        >
          {error.message}
        </div>
      )}

      <div className="grid flex-1 grid-cols-1 gap-4 overflow-hidden lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <section className="flex flex-col gap-4 overflow-y-auto pr-1">
          <ThemeGallery presets={presets} />
          <Tabs defaultValue="tokens">
            <TabsList>
              <TabsTrigger value="tokens">Tokens</TabsTrigger>
              <TabsTrigger value="branding">Branding</TabsTrigger>
            </TabsList>
            <TabsContent value="tokens" className="pt-4">
              <ColorTokenEditor />
            </TabsContent>
            <TabsContent value="branding" className="pt-4">
              <BrandingEditor
                hideFeatureOptions={hideFeatureOptions}
                logoUrl={logoUrl}
                faviconUrl={faviconUrl}
              />
            </TabsContent>
          </Tabs>
        </section>

        <section className="overflow-hidden">
          <LivePreview className="h-full" />
        </section>
      </div>
    </div>
  );
}
