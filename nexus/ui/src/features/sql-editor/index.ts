// Reusable SQL editing surface, shared by Explore and the dashboard panel
// editors: a CodeMirror Postgres editor with datasource-schema-aware
// autocomplete, plus the hook that learns and caches a datasource's schema.
export { SqlEditor } from "@/features/sql-editor/SqlEditor";
export { useDatasourceSchema } from "@/features/sql-editor/useDatasourceSchema";
