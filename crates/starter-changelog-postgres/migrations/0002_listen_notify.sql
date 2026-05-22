-- starter-changelog Postgres — LISTEN/NOTIFY trigger.
--
-- Publishes a NOTIFY on `starter_changes_new` whenever a row is
-- appended to `starter_changes`. The payload is the row id; tails
-- treat the notification only as a wakeup signal and SELECT any
-- rows newer than their cursor (so coalesced or dropped
-- notifications never cause missed changes).
--
-- See `DOCS/backend/undo-redo/SCOPE.md` §"Storage shape".

CREATE OR REPLACE FUNCTION starter_changes_notify() RETURNS trigger AS $$
BEGIN
    -- pg_notify silently truncates payloads > 8000 bytes; row ids
    -- are short (ULIDs / UUIDs) so this is safe.
    PERFORM pg_notify('starter_changes_new', NEW.id);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS starter_changes_notify_trg ON starter_changes;
CREATE TRIGGER starter_changes_notify_trg
    AFTER INSERT ON starter_changes
    FOR EACH ROW EXECUTE FUNCTION starter_changes_notify();
