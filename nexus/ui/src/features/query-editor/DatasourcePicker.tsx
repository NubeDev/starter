import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { useDatasources } from "@/features/datasources/useDatasources";

// Datasource selector backed by the real `GET /datasources`. While the
// list loads or if it's empty the trigger says so rather than offering a
// fabricated option (F0). The selected id is lifted to the caller (the
// Explorer / panel config owns the query).
export function DatasourcePicker({
  value,
  onChange,
}: {
  value: string | undefined;
  onChange: (id: string) => void;
}) {
  const { data, isPending, isError } = useDatasources();

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
