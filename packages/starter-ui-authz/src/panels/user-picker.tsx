// `<UserPicker>` — searchable subject picker.
//
// Replaces the free-text subject/user-id inputs in the
// Assignments, Members, and Teams panels. Consumes a host-provided
// `UserDirectory` adapter via context so this package does not
// depend on `@nube/rubix-client-react` directly.
//
// Emits a discriminated `{kind, id, label}` on selection:
//   - user mode  -> id is the rubix user_id, label is the email
//   - team mode  -> id is the synthetic subject "team:<slug>",
//                   label is the team display name or slug
//   - glob mode  -> id is the raw glob (e.g. "user-*"),
//                   label is the same raw string

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Button,
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  Input,
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@nube/starter-ui-kit";

export interface UserDirectoryEntry {
  user_id: string;
  email: string;
  role?: string;
  disabled_at_ms?: number | null;
}

export interface UserDirectory {
  search(query: string): Promise<UserDirectoryEntry[]> | UserDirectoryEntry[];
  getById?(id: string): UserDirectoryEntry | undefined;
}

export interface UserPickerTeam {
  id: string;
  slug: string;
  displayName?: string;
}

export type UserPickerSelection =
  | { kind: "user"; id: string; label: string }
  | { kind: "team"; id: string; label: string }
  | { kind: "glob"; id: string; label: string };

export interface UserPickerProps {
  value: string | null;
  onChange: (next: UserPickerSelection | null) => void;
  userDirectory: UserDirectory;
  teams?: UserPickerTeam[];
  enableTeamMode?: boolean;
  enableGlobMode?: boolean;
  placeholder?: string;
  disabled?: boolean;
  id?: string;
}

const UserDirectoryContext = createContext<UserDirectory | null>(null);

export interface UserDirectoryProviderProps {
  value: UserDirectory | null | undefined;
  children: React.ReactNode;
}

export function UserDirectoryProvider({ value, children }: UserDirectoryProviderProps) {
  return (
    <UserDirectoryContext.Provider value={value ?? null}>
      {children}
    </UserDirectoryContext.Provider>
  );
}

let warnedMissingDirectory = false;
export function useUserDirectory(): UserDirectory | null {
  const dir = useContext(UserDirectoryContext);
  useEffect(() => {
    if (!dir && !warnedMissingDirectory) {
      warnedMissingDirectory = true;

      console.warn(
        "[starter-ui-authz] No userDirectory provided to <AuthzAdmin>; <UserPicker> falls back to a plain text input.",
      );
    }
  }, [dir]);
  return dir;
}

function parseValue(value: string | null): { kind: "user" | "team" | "glob"; raw: string } | null {
  if (!value) return null;
  if (value.startsWith("team:")) return { kind: "team", raw: value.slice("team:".length) };
  if (value.includes("*") || value.includes("?")) return { kind: "glob", raw: value };
  return { kind: "user", raw: value };
}

export function UserPicker(props: UserPickerProps) {
  const {
    value,
    onChange,
    userDirectory,
    teams,
    enableTeamMode = false,
    enableGlobMode = true,
    placeholder,
    disabled,
    id,
  } = props;

  const parsed = useMemo(() => parseValue(value), [value]);
  const initialMode = parsed?.kind ?? "user";

  const [mode, setMode] = useState<"user" | "team" | "glob">(initialMode);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [results, setResults] = useState<UserDirectoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(initialMode === "glob");
  const [globDraft, setGlobDraft] = useState(initialMode === "glob" ? (parsed?.raw ?? "") : "");

  // Debounce ~200ms.
  useEffect(() => {
    const h = setTimeout(() => setDebounced(query), 200);
    return () => clearTimeout(h);
  }, [query]);

  // Run search when debounced query changes (user mode).
  const reqId = useRef(0);
  useEffect(() => {
    if (mode !== "user") return;
    const my = ++reqId.current;
    setLoading(true);
    const out = userDirectory.search(debounced);
    Promise.resolve(out)
      .then((rows) => {
        if (my !== reqId.current) return;
        setResults(rows);
      })
      .catch(() => {
        if (my !== reqId.current) return;
        setResults([]);
      })
      .finally(() => {
        if (my === reqId.current) setLoading(false);
      });
  }, [debounced, mode, userDirectory]);

  const resolvedLabel = useMemo(() => {
    if (!parsed) return "";
    if (parsed.kind === "user") {
      const hit = userDirectory.getById?.(parsed.raw);
      return hit?.email ?? parsed.raw;
    }
    if (parsed.kind === "team") {
      const t = teams?.find((x) => x.slug === parsed.raw || x.id === parsed.raw);
      return t?.displayName ?? t?.slug ?? `team:${parsed.raw}`;
    }
    return parsed.raw;
  }, [parsed, userDirectory, teams]);

  const triggerLabel = resolvedLabel || (placeholder ?? "Pick a subject…");

  const segments: Array<{ key: "user" | "team" | "glob"; label: string; visible: boolean }> = [
    { key: "user", label: "User", visible: true },
    { key: "team", label: "Team", visible: !!enableTeamMode },
    { key: "glob", label: "Glob", visible: !!enableGlobMode && showAdvanced },
  ];

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          type="button"
          variant="outline"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className="w-full justify-between font-normal"
        >
          <span className="truncate">{triggerLabel}</span>
          <span className="ml-2 text-xs opacity-60">▼</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[--radix-popover-trigger-width] min-w-[20rem] p-0" align="start">
        <div className="flex items-center gap-1 border-b px-2 py-1.5">
          {segments
            .filter((s) => s.visible)
            .map((s) => (
              <Button
                key={s.key}
                type="button"
                size="sm"
                variant={mode === s.key ? "default" : "ghost"}
                onClick={() => setMode(s.key)}
                className="h-7 px-2 text-xs"
              >
                {s.label}
              </Button>
            ))}
          <div className="ml-auto">
            {enableGlobMode && !showAdvanced ? (
              <Button
                type="button"
                size="sm"
                variant="ghost"
                onClick={() => setShowAdvanced(true)}
                className="h-7 px-2 text-xs"
              >
                Advanced
              </Button>
            ) : null}
          </div>
        </div>

        {mode === "user" ? (
          <Command shouldFilter={false}>
            <CommandInput
              placeholder="Search users by email or id…"
              value={query}
              onValueChange={setQuery}
            />
            <CommandList>
              {loading ? (
                <div className="px-3 py-2 text-xs opacity-60">Searching…</div>
              ) : results.length === 0 ? (
                <CommandEmpty>No matches.</CommandEmpty>
              ) : (
                <CommandGroup>
                  {results.map((u) => (
                    <CommandItem
                      key={u.user_id}
                      value={`${u.email} ${u.user_id}`}
                      onSelect={() => {
                        onChange({ kind: "user", id: u.user_id, label: u.email });
                        setOpen(false);
                      }}
                    >
                      <div className="flex w-full items-center justify-between gap-3">
                        <span className="truncate">{u.email}</span>
                        <span className="shrink-0 text-xs opacity-60">
                          {u.role ? `${u.role} — ` : ""}
                          <code>{u.user_id}</code>
                        </span>
                      </div>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        ) : null}

        {mode === "team" ? (
          <Command shouldFilter>
            <CommandInput placeholder="Search teams…" />
            <CommandList>
              {(teams ?? []).length === 0 ? (
                <CommandEmpty>No teams.</CommandEmpty>
              ) : (
                <CommandGroup>
                  {(teams ?? []).map((t) => (
                    <CommandItem
                      key={t.id}
                      value={`${t.slug} ${t.displayName ?? ""}`}
                      onSelect={() => {
                        onChange({
                          kind: "team",
                          id: `team:${t.slug}`,
                          label: t.displayName ?? t.slug,
                        });
                        setOpen(false);
                      }}
                    >
                      <div className="flex w-full items-center justify-between gap-3">
                        <span className="truncate">{t.displayName ?? t.slug}</span>
                        <span className="shrink-0 text-xs opacity-60">
                          <code>{t.slug}</code>
                        </span>
                      </div>
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        ) : null}

        {mode === "glob" ? (
          <div className="grid gap-2 p-3">
            <p className="text-xs opacity-70">
              Advanced: enter a glob pattern (e.g. <code>user-*</code>).
            </p>
            <Input
              autoFocus
              value={globDraft}
              onChange={(e) => setGlobDraft(e.currentTarget.value)}
              placeholder="user-*"
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  const raw = globDraft.trim();
                  if (!raw) return;
                  onChange({ kind: "glob", id: raw, label: raw });
                  setOpen(false);
                }
              }}
            />
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                size="sm"
                variant="ghost"
                onClick={() => {
                  onChange(null);
                  setGlobDraft("");
                  setOpen(false);
                }}
              >
                Clear
              </Button>
              <Button
                type="button"
                size="sm"
                onClick={() => {
                  const raw = globDraft.trim();
                  if (!raw) return;
                  onChange({ kind: "glob", id: raw, label: raw });
                  setOpen(false);
                }}
              >
                Use pattern
              </Button>
            </div>
          </div>
        ) : null}
      </PopoverContent>
    </Popover>
  );
}

/**
 * Fallback plain-text picker — rendered when no `UserDirectory` is
 * available in context. Mirrors the original free-text input so
 * existing consumers keep working.
 */
export function UserPickerFallback(props: {
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
  id?: string;
  required?: boolean;
  disabled?: boolean;
}) {
  return (
    <Input
      id={props.id}
      value={props.value}
      onChange={(e) => props.onChange(e.currentTarget.value)}
      placeholder={props.placeholder}
      required={props.required}
      disabled={props.disabled}
    />
  );
}
