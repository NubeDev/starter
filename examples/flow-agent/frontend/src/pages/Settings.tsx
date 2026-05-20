import { Card, CardContent, CardHeader, CardTitle } from "@nube/starter-ui-kit";

export function Settings() {
  return (
    <div className="mx-auto w-full max-w-3xl p-6">
      <h1 className="mb-4 text-2xl font-semibold tracking-tight">Settings</h1>
      <Card className="border-border/60 shadow-sm">
        <CardHeader>
          <CardTitle className="text-base">Providers</CardTitle>
        </CardHeader>
        <CardContent className="text-sm text-muted-foreground">
          Provider keys are read from environment variables
          (<code>ANTHROPIC_API_KEY</code>, <code>OPENAI_API_KEY</code>) or
          from the Claude CLI session. A UI for managing them lands later.
        </CardContent>
      </Card>
    </div>
  );
}
