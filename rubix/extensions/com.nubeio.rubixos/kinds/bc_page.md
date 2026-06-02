# `com.nubeio.rubixos.bc_page`

`bc_page_create` inserts a dashboard page that provisioned devices
hang their widgets off. Upsert on `page_id`, tenant-scoped.

A page belongs to a site (`site_id`) and, optionally, a location within
it (`location_id`) — clients browse "Site → Location → its pages". Both
are optional in `row`; omit `location_id` for a site-level page.

`bc_page_update` changes a page by `page_id` (the only required key in
`row`): rename via `name`, or re-pin via `site_id` / `location_id`.

`bc_page_delete` removes a page by `page_id`. Its dashboard widgets are
deleted, and any devices placed on it are **kept** but detached — their
`page_id` is cleared and `status` flips to `pending` (unprovisioned), so
they resurface in the unassigned list and can be re-placed. The result
reports `affected`, `widgets_deleted`, and `devices_detached`.
