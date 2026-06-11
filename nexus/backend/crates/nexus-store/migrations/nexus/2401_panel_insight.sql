-- RW-06: let a dashboard panel carry an optional post-query insight, completing
-- the link 2101_insights.sql already anticipated ("Panels reference an insight by
-- id"). The panel keeps owning its SQL + datasource; the insight is the transform
-- applied to that query's result before it reaches the widget.
--
-- `insight_id` is a nullable FK to nexus_insights with ON DELETE SET NULL — the
-- same posture as a panel's datasource: deleting the referenced insight detaches
-- it rather than cascading the panel away, so the panel keeps rendering its raw
-- query. RLS already isolates both tables per tenant; the FK is within-tenant by
-- construction (a panel and the insight it names share the row's tenant_id, and
-- neither table is visible cross-tenant).
--
-- `insight_params` is the optional JSON bound as the script's `params` object at
-- query time (mirrors InsightRef.params). Nullable = "no params".

ALTER TABLE nexus_panels
    ADD COLUMN insight_id     uuid REFERENCES nexus_insights(id) ON DELETE SET NULL,
    ADD COLUMN insight_params jsonb;
