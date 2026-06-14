import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { useCan } from "@/auth/useCan";
import { NEXUS_DB_DATASOURCE_ID } from "@/api/nexus-db/query";
import { useDatasources } from "@/features/datasources/useDatasources";

// Datasource selector backed by the real `GET /datasources`. While the
// list loads or if it's empty the trigger says so rather than offering a
// fabricated option (F0). The selected id is lifted to the caller (the
// Explorer / panel config owns the query).
//
// `includeNexusDb` appends a virtual "Nexus DB" entry (the control-plane DB)
// using the `NEXUS_DB_DATASOURCE_ID` sentinel rather than a real datasource
// row. It's admin-only (the endpoint 403s otherwise) and only offered when the
// caller opts in — e.g. a dashboard panel that wants to read platform internals.
export function DatasourcePicker({
  value,
  onChange,
  includeNexusDb = false,
}: {
  value: string | undefined;
  onChange: (id: string) => void;
  includeNexusDb?: boolean;
}) {
  const { data, isPending, isError } = useDatasources();
  const isAdmin = useCan("admin");
  const showNexusDb = includeNexusDb && isAdmin;

  const placeholder = isPending
    ? "Loading datasources…"
    : isError
      ? "Failed to load datasources"
      : data && data.length === 0
        ? "No datasources"
        : "Select a datasource";

  return (
    <Select value={value} onValueChange={onChange} disabled={isPending || isError}>
      <SelectTrigger className="w-64">
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {showNexusDb ? (
          <SelectItem value={NEXUS_DB_DATASOURCE_ID}>
            Nexus DB
            <span className="ms-2 text-xs text-muted-foreground">
              control-plane · read-only
            </span>
          </SelectItem>
        ) : null}
        {(data ?? []).map((ds) => (
          <SelectItem key={ds.id} value={ds.id}>
            {ds.name}
            <span className="ms-2 text-xs text-muted-foreground">{ds.kind}</span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
