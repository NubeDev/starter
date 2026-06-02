# `com.nubeio.rubixos.bc_page`

`bc_page_create` inserts a dashboard page that provisioned devices
hang their widgets off. Upsert on `page_id`, tenant-scoped.

A page belongs to a site (`site_id`) and, optionally, a location within
it (`location_id`) — clients browse "Site → Location → its pages". Both
are optional in `row`; omit `location_id` for a site-level page.
