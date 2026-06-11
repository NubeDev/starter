import { useParams } from "react-router-dom";

import { ExtensionSlot } from "@/extensions/ExtensionSlot";

// Per-extension page host (the rubix `/extensions/$extId/$` mechanism, ported).
//
// Route: `/x/:extId/*`. The splat after the extension id is forwarded to the
// mounted contribution as `SlotContext.route`, so the extension's `Main`
// component (exposed at `slot: main`) can dispatch its own sub-pages with
// `useExtensionRoute()` — exactly how rubix's `MainRouter` works.
//
// `extensionId` filters the `main` slot to just this extension, so two
// extensions both contributing `slot: main` don't stack on one URL.
export function ExtensionPage() {
  const params = useParams();
  const extId = params.extId ?? "";
  // react-router gives the splat under the "*" param.
  const route = params["*"] ?? "";

  return (
    <ExtensionSlot id="main" extensionId={extId} route={route} />
  );
}
