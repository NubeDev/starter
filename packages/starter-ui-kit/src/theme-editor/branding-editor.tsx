// Shell branding panel. Edits the `nav_title` and `hide_features`
// fields plus the pending logo and favicon uploads.
//
// `hideFeatureOptions` is consumer-supplied: starter has no opinion on
// what features a given product exposes. If omitted, the editor just
// shows a free-text editor for the `hide_features` array — that
// works for any string identifier the consumer picks.

import { useCallback, useId, useRef } from "react";

import { useThemeEditorStore } from "@nube/starter-ui-core/theme-editor";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

export interface BrandingEditorProps {
  /** Optional consumer-defined feature flags the admin can toggle off.
   * If empty/omitted, the section is hidden. */
  hideFeatureOptions?: ReadonlyArray<{ id: string; label: string }>;
  /** Server-stored logo URL (from `useThemeEditor`). Used as the
   * default preview when no upload is pending. */
  logoUrl?: string | null;
  /** Server-stored favicon URL (from `useThemeEditor`). */
  faviconUrl?: string | null;
  className?: string;
}

const LOGO_MAX_BYTES = 256 * 1024;
const FAVICON_MAX_BYTES = 64 * 1024;
const LOGO_TYPES = ["image/png", "image/svg+xml"];
const FAVICON_TYPES = ["image/png", "image/x-icon", "image/vnd.microsoft.icon"];

export function BrandingEditor({
  hideFeatureOptions = [],
  logoUrl,
  faviconUrl,
  className,
}: BrandingEditorProps) {
  const navTitle = useThemeEditorStore((s) => s.shell.nav_title);
  const hideFeatures = useThemeEditorStore((s) => s.shell.hide_features);
  const setShellField = useThemeEditorStore((s) => s.setShellField);
  const navTitleId = useId();

  const toggleFeature = useCallback(
    (id: string, on: boolean) => {
      const next = on
        ? Array.from(new Set([...hideFeatures, id]))
        : hideFeatures.filter((f) => f !== id);
      setShellField("hide_features", next);
    },
    [hideFeatures, setShellField],
  );

  return (
    <div className={cn("flex flex-col gap-6", className)}>
      <section className="flex flex-col gap-2">
        <Label htmlFor={navTitleId}>Navigation title</Label>
        <Input
          id={navTitleId}
          value={navTitle}
          onChange={(e) => setShellField("nav_title", e.target.value)}
          placeholder="My App"
        />
        <p className="text-xs text-muted-foreground">
          Shown in the app header next to the logo. Leave blank to use the consumer-defined default.
        </p>
      </section>

      {hideFeatureOptions.length > 0 && (
        <section className="flex flex-col gap-2">
          <h3 className="text-sm font-semibold">Hide features</h3>
          {hideFeatureOptions.map((opt) => {
            const checked = hideFeatures.includes(opt.id);
            return (
              <label key={opt.id} className="flex items-center gap-2 text-sm">
                <Checkbox
                  checked={checked}
                  onCheckedChange={(v) => toggleFeature(opt.id, v === true)}
                />
                {opt.label}
              </label>
            );
          })}
        </section>
      )}

      <AssetDropZone
        label="Logo"
        accept={LOGO_TYPES}
        maxBytes={LOGO_MAX_BYTES}
        currentUrl={logoUrl ?? null}
        getter={(s) => s.pendingLogo}
        setter={(f) => useThemeEditorStore.getState().setPendingLogo(f)}
        hint="PNG or SVG, max 256 KiB."
      />

      <AssetDropZone
        label="Favicon"
        accept={FAVICON_TYPES}
        maxBytes={FAVICON_MAX_BYTES}
        currentUrl={faviconUrl ?? null}
        getter={(s) => s.pendingFavicon}
        setter={(f) => useThemeEditorStore.getState().setPendingFavicon(f)}
        hint="PNG or ICO, max 64 KiB."
      />
    </div>
  );
}

interface AssetDropZoneProps {
  label: string;
  accept: readonly string[];
  maxBytes: number;
  currentUrl: string | null;
  /** Read the pending file from the store. */
  getter: (state: ReturnType<typeof useThemeEditorStore.getState>) => File | null | undefined;
  /** Write the pending file (or `undefined` to delete on save). */
  setter: (file: File | null | undefined) => void;
  hint: string;
}

function AssetDropZone({ label, accept, maxBytes, currentUrl, getter, setter, hint }: AssetDropZoneProps) {
  const pending = useThemeEditorStore(getter);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const fieldId = useId();

  const handleFile = useCallback(
    (file: File | null) => {
      if (!file) {
        setter(null);
        return;
      }
      if (!accept.includes(file.type)) {
        // eslint-disable-next-line no-alert
        alert(`${label} must be one of: ${accept.join(", ")}`);
        return;
      }
      if (file.size > maxBytes) {
        // eslint-disable-next-line no-alert
        alert(`${label} exceeds ${maxBytes / 1024} KiB`);
        return;
      }
      setter(file);
    },
    [accept, maxBytes, label, setter],
  );

  const previewUrl =
    pending instanceof File ? URL.createObjectURL(pending) : pending === undefined ? null : currentUrl;

  return (
    <section className="flex flex-col gap-2">
      <Label htmlFor={fieldId}>{label}</Label>
      <div
        className="flex items-center gap-3 rounded-lg border border-dashed border-border p-3"
        onDragOver={(e) => {
          e.preventDefault();
        }}
        onDrop={(e) => {
          e.preventDefault();
          const f = e.dataTransfer.files?.[0];
          if (f) handleFile(f);
        }}
      >
        {previewUrl ? (
          <img src={previewUrl} alt={`${label} preview`} className="size-12 rounded border border-border bg-background object-contain" />
        ) : (
          <div className="size-12 rounded border border-border bg-muted" aria-hidden />
        )}
        <div className="flex-1">
          <input
            id={fieldId}
            ref={inputRef}
            type="file"
            accept={accept.join(",")}
            className="hidden"
            onChange={(e) => handleFile(e.target.files?.[0] ?? null)}
          />
          <p className="text-xs text-muted-foreground">{hint}</p>
        </div>
        <div className="flex gap-2">
          <Button type="button" size="sm" variant="outline" onClick={() => inputRef.current?.click()}>
            Choose…
          </Button>
          {(previewUrl || currentUrl) && (
            <Button
              type="button"
              size="sm"
              variant="destructive"
              onClick={() => setter(undefined)}
              title={`Delete the saved ${label.toLowerCase()} on next save`}
            >
              Remove
            </Button>
          )}
        </div>
      </div>
    </section>
  );
}
