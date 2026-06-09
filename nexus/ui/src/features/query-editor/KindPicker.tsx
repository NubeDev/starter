import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { listQueryKinds } from "@/api/query/kinds";

// Query-kind selector (WS-10): backed by the real `GET /query/kinds`, it lists
// the declarative queries a panel can invoke by reverse-DNS name instead of raw
// SQL. The selected kind name is lifted to the caller, which sends it as the
// `kind` field of a `QueryRequest`. While the catalogue loads or if it's empty
// the trigger says so rather than offering a fabricated option (F0). This is
// the minimal picker; the schema-driven params form is a WS-04 follow-up.
export function KindPicker({
  value,
  onChange,
}: {
  value: string | undefined;
  onChange: (name: string) => void;
}) {
  const client = useStarterClient();
  const { data, isPending, isError } = useQuery({
    queryKey: ["query-kinds"],
    queryFn: () => listQueryKinds(client),
  });

  const kinds = data?.kinds ?? [];
  const placeholder = isPending
    ? "Loading kinds…"
    : isError
      ? "Failed to load kinds"
      : kinds.length === 0
        ? "No kinds"
        : "Select a kind";

  return (
    <Select value={value} onValueChange={onChange} disabled={isPending || isError}>
      <SelectTrigger className="w-72">
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {kinds.map((k) => (
          <SelectItem key={k.name} value={k.name}>
            {k.name}
            {k.description ? (
              <span className="ms-2 text-xs text-muted-foreground">
                {k.description}
              </span>
            ) : null}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
