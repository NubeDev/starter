# sites.register_site

Register a site by `site_name` + `address`. Resolves the address to coordinates
by calling the `com.acme.geocode.lookup` peer (synchronous `extension.call`),
then publishes `{ site_name, address, lat, lon }` on `com.acme.sites.registered`
(async event-bus publish) so other same-tenant surfaces can live-update.
