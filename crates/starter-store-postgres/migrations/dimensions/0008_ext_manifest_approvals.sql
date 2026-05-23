-- Warehouse SCOPE W12: extension manifest approval trust seam.
-- Composite (ext_id, manifest_hash) PK so the installer can record
-- multiple historical approvals per extension across upgrades.

CREATE TABLE IF NOT EXISTS ext_manifest_approvals (
    ext_id        TEXT NOT NULL,
    manifest_hash TEXT NOT NULL,
    approved_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_by   TEXT NOT NULL,
    PRIMARY KEY (ext_id, manifest_hash),
    CONSTRAINT ext_manifest_approvals_approved_by_valid CHECK (
        approved_by LIKE 'user:%' OR approved_by LIKE 'install:%'
    )
);
