-- File-backed datasource persistence (RW-04 fix pass).
--
-- Migration 0001 shaped `nexus_datasources` for a credentialed SQL connector:
-- host/port/database/db_user and the four envelope-secret columns are all NOT
-- NULL. A file datasource (parquet/csv) has none of those — its only config is a
-- server-local path and it holds no secret — so it could not be stored at all.
--
-- This relaxes the connection + secret columns to nullable and adds a generic
-- `config jsonb` carrying the non-SQL shape (`{path, has_header}` for file kinds).
-- Existing Postgres rows are unaffected: they keep their populated columns and a
-- NULL `config`. New SQL datasources still populate the connection/secret columns;
-- only secret-less file kinds leave them NULL and fill `config`.

ALTER TABLE nexus_datasources
    ALTER COLUMN host DROP NOT NULL,
    ALTER COLUMN port DROP NOT NULL,
    ALTER COLUMN database DROP NOT NULL,
    ALTER COLUMN db_user DROP NOT NULL,
    ALTER COLUMN secret_cipher DROP NOT NULL,
    ALTER COLUMN secret_nonce DROP NOT NULL,
    ALTER COLUMN wrapped_data_key DROP NOT NULL,
    ALTER COLUMN data_key_nonce DROP NOT NULL,
    ADD COLUMN config jsonb;
