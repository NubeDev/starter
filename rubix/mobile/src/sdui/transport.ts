// sdui/transport.ts — build an SduiTransport from a StarterClient.
//
// Plain wrapper around `createHttpSduiTransport`. The mobile app
// rebuilds this whenever the active connection changes (see
// connection/provider.tsx) so a transport never outlives the client
// whose baseUrl + bearer it was built against.

import type { StarterClient } from '@nube/starter-client-ts';
import {
  createHttpSduiTransport,
  type SduiTransport,
} from '@nube/starter-ui-sdui-react/headless';

export function makeSduiTransport(client: StarterClient): SduiTransport {
  return createHttpSduiTransport({ client });
}
