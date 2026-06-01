# `com.nubeio.rubixos.bc_template_upsert`

Validate and store a YAML device template at runtime. The submitted
YAML is parsed and checked, then persisted into `bc_templates`
(decomposed into `points_json` and `widget_group_json`) so that
later scans of matching models can provision against it. Upsert on
`(template, version)`, tenant-scoped.
