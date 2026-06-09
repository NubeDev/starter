import { Empty } from "@/features/state/Empty";

// Placeholder for alert rules / events. The backend is building the alert
// model (`nexus-store::alert` — rules, channels, events) but it isn't in
// the OpenAPI contract yet, so there are no bindings to call. This screen
// renders an honest "not available" state rather than mock rules (F0).
// When the contract surfaces `/alerts`, this becomes a real
// list/create/acknowledge view over the codegen'd client.
export function AlertsPage() {
  return (
    <Empty
      title="Alerts are coming soon"
      description="Rule and notification management will appear here once the backend exposes the alerts API."
    />
  );
}
