# `com.nubeio.rubixos.bc_site`

Topology tools for the provisioning hierarchy. `bc_site_create`
inserts a site; `bc_location_create` inserts a location bound to a
parent site. Both are upserts on their natural id and are scoped to
the caller's tenant.
