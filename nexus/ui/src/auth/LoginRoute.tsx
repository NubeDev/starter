import { useState, type FormEvent } from "react";
import { useAuth } from "@nube/starter-client-react";
import { StarterError } from "@nube/starter-client-ts";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@nube/starter-ui-kit/components/card";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

// Rendered by `<AuthProvider>` as its `unauthenticatedSlot` whenever
// `me()` returns 401. Acting as a slot (not a route) keeps the user's
// target URL in the address bar; after login the provider re-probes
// `me()` and swaps back to the routed app.
export function LoginRoute() {
  const auth = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await auth.login({ email, password });
    } catch (err) {
      setError(
        err instanceof StarterError
          ? err.status === 401
            ? "Invalid email or password."
            : `Login failed (${err.status}).`
          : "Login failed. Please try again.",
      );
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center p-6">
      <Card className="glass w-full max-w-sm">
        <CardHeader>
          <CardTitle>Nexus</CardTitle>
          <CardDescription>Sign in to your dashboards.</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="space-y-4" onSubmit={onSubmit}>
            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                autoComplete="username"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {error ? (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            ) : null}
            <Button type="submit" className="w-full" disabled={submitting}>
              {submitting ? "Signing in…" : "Sign in"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
