import { useState } from "react";
import { Sparkles } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@nube/starter-ui-kit/components/popover";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import { resultSql, useAssist } from "@/features/ai/useAssist";

// "Ask AI" toolbar control: a popover with a plain-English prompt that
// generates SQL via POST /ai/assist (task "sql"). The selected datasource
// grounds the model with the real schema and the editor's current SQL lets
// it edit/improve rather than start from scratch. On success the generated
// SQL is handed back through onApply — this component never runs the query;
// the user still reviews and runs it with the existing Run/Test buttons.
export function AiSqlAssist({
  datasourceId,
  currentSql,
  onApply,
}: {
  datasourceId?: string;
  currentSql?: string;
  onApply: (sql: string) => void;
}) {
  const assist = useAssist();
  const [open, setOpen] = useState(false);
  const [prompt, setPrompt] = useState("");

  const canGenerate = prompt.trim().length > 0 && !assist.isPending;

  const generate = () => {
    if (!canGenerate) return;
    assist.mutate(
      {
        task: "sql",
        prompt: prompt.trim(),
        datasource_id: datasourceId,
        current_sql: currentSql,
      },
      {
        onSuccess: (res) => {
          onApply(resultSql(res));
          setOpen(false);
          setPrompt("");
          assist.reset();
        },
      },
    );
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button type="button" variant="outline" size="sm" className="gap-2">
          <Sparkles className="size-4" />
          Ask AI
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <div className="flex flex-col gap-2">
          <span className="text-xs font-medium text-muted-foreground">
            Describe the query you want
          </span>
          <Textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="e.g. average temperature per site over the last 24 hours"
            rows={3}
            aria-label="Describe the query in plain English"
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === "Enter") generate();
            }}
          />
          {assist.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {assist.error instanceof Error
                ? assist.error.message
                : "Couldn't generate SQL."}
            </p>
          ) : null}
          <Button
            type="button"
            size="sm"
            className="w-full gap-2"
            disabled={!canGenerate}
            onClick={generate}
          >
            <Sparkles className="size-4" />
            {assist.isPending ? "Generating…" : "Generate"}
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
