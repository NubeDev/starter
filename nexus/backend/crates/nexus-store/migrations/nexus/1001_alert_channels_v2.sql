-- Alerting v2: multi-condition rules, a no-data/error policy, and a per-rule
-- message template. Purely additive to nexus_alert_rules — the existing
-- single-condition columns (query/op/threshold) stay, and a NULL `conditions`
-- means "use the legacy single condition built from those columns", so every
-- rule created before this migration evaluates exactly as before.

-- The ordered list of conditions, each {query, reducer, op, threshold}, combined
-- by `combinator`. NULL = the legacy single-condition path. A reducer collapses a
-- condition's query rows to one value (last|min|max|avg|sum|count) before the
-- comparison, so a condition can aggregate a result set, not just read row one.
ALTER TABLE nexus_alert_rules
    ADD COLUMN conditions jsonb,
    -- How the conditions combine: and|or. Defaults to the stricter `and`.
    ADD COLUMN combinator text NOT NULL DEFAULT 'and',
    -- What a no-data evaluation resolves to: ok|alerting|keep_last.
    ADD COLUMN no_data_policy text NOT NULL DEFAULT 'ok',
    -- What an execution error resolves to: ok|alerting|keep_last.
    ADD COLUMN exec_error_policy text NOT NULL DEFAULT 'ok',
    -- Optional per-rule notification message template; NULL uses the default.
    ADD COLUMN message_template text;
