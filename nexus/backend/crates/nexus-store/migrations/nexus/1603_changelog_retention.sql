-- Audit-ledger retention (WS-12) — bounded growth for the append-only log.
--
-- The change ledger is append-only, so without pruning it grows without bound.
-- A background sweep (nexus-api `changelog::prune`) deletes rows older than the
-- retention horizon. Like the alert scheduler's claim, the sweep is a system
-- actor that must act across every tenant, which RLS (correctly) forbids the
-- runtime role from doing directly. Rather than grant BYPASSRLS, this
-- SECURITY DEFINER function — owned by the migration role — exposes exactly one
-- controlled cross-tenant write: delete ledger rows whose `at` is older than the
-- given cutoff, capped at `batch` rows per call so one sweep cannot lock the
-- table for an unbounded delete. It returns the number of rows deleted, so the
-- sweep loops until a call deletes fewer than `batch`.
--
-- Retention is a single global horizon, not per-kind: a coarse, auditable policy
-- (one number an operator can reason about) beats a per-kind matrix nothing yet
-- needs. Per-kind horizons are a fast-follow if a kind ever needs a different
-- one — the function would take a kind filter, the caller a policy table.
CREATE FUNCTION nexus_prune_changes(cutoff timestamptz, batch integer)
RETURNS bigint
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    deleted bigint;
BEGIN
    WITH aged AS (
        SELECT id
        FROM nexus_changes
        WHERE at < cutoff
        ORDER BY at
        LIMIT batch
        FOR UPDATE SKIP LOCKED
    )
    DELETE FROM nexus_changes c
    USING aged
    WHERE c.id = aged.id;
    GET DIAGNOSTICS deleted = ROW_COUNT;
    RETURN deleted;
END;
$$;
-- Only the runtime role may invoke it; the definer rights are scoped to the one
-- delete it performs.
REVOKE ALL ON FUNCTION nexus_prune_changes(timestamptz, integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION nexus_prune_changes(timestamptz, integer) TO nexus_runtime;
