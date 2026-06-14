import { useState, type FormEvent } from "react";
import { Plus, X } from "lucide-react";
import { Badge } from "@nube/starter-ui-kit/components/badge";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";

import type { Tag, TaggableKind } from "@/api/types";
import { useSetTags, useTagKeys, useTags } from "@/features/tags/useTags";

// Reusable tag editor for any entity. Drop it onto a detail view with the
// entity's kind + id; it loads the current tags, shows them as removable
// chips, and persists the whole set on every change (the API is a full
// replace). Handles both shapes: a bare label (`temp`) and a key:value tag
// (`building=abc`).
export function TagEditor({
  kind,
  id,
}: {
  kind: TaggableKind;
  id: string;
}) {
  const { data: tags, isPending, isError } = useTags(kind, id);
  const { data: keySuggestions } = useTagKeys();
  const save = useSetTags(kind, id);

  const [key, setKey] = useState("");
  const [value, setValue] = useState("");

  if (isPending) {
    return <p className="text-xs text-muted-foreground">Loading tags…</p>;
  }
  if (isError) {
    return <p className="text-xs text-destructive">Couldn't load tags.</p>;
  }

  // After the guards above, the data is loaded.
  const current: Tag[] = tags;

  // Replace the set with `next` — the API has no add/remove, only set.
  const persist = (next: Tag[]) => save.mutate({ tags: next });

  function onAdd(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const k = key.trim();
    if (!k) return;
    const v = value.trim();
    const tag: Tag = v ? { key: k, value: v } : { key: k };
    // Upsert by key: a repeated key replaces its value rather than duplicating,
    // matching the backend's unique-per-key constraint.
    const next = [...current.filter((t) => t.key !== k), tag];
    persist(next);
    setKey("");
    setValue("");
  }

  const remove = (k: string) => persist(current.filter((t) => t.key !== k));

  const listId = `tag-keys-${kind}-${id}`;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap gap-1.5">
        {current.length === 0 ? (
          <span className="text-xs text-muted-foreground">No tags</span>
        ) : (
          current.map((t) => (
            <Badge key={t.key} variant="secondary" className="gap-1">
              <span className="font-medium">{t.key}</span>
              {t.value != null ? (
                <span className="text-muted-foreground">={t.value}</span>
              ) : null}
              <button
                type="button"
                aria-label={`Remove tag ${t.key}`}
                className="ms-0.5 rounded-sm text-muted-foreground hover:text-destructive"
                disabled={save.isPending}
                onClick={() => remove(t.key)}
              >
                <X className="size-3" />
              </button>
            </Badge>
          ))
        )}
      </div>

      <form className="flex items-center gap-2" onSubmit={onAdd}>
        <Input
          aria-label="Tag key"
          placeholder="key (e.g. building)"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          list={listId}
          className="h-8 w-40"
        />
        <datalist id={listId}>
          {(keySuggestions ?? []).map((k) => (
            <option key={k} value={k} />
          ))}
        </datalist>
        <span className="text-muted-foreground">=</span>
        <Input
          aria-label="Tag value (optional)"
          placeholder="value (optional)"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          className="h-8 w-40"
        />
        <Button
          type="submit"
          size="sm"
          variant="outline"
          className="gap-1"
          disabled={save.isPending || key.trim() === ""}
        >
          <Plus className="size-4" />
          Add
        </Button>
      </form>
      {save.isError ? (
        <p role="alert" className="text-xs text-destructive">
          Couldn't save tags.
        </p>
      ) : null}
    </div>
  );
}
