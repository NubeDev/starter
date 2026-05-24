-- Reverse 0002_history. Pure DROP; the `IF EXISTS` keeps re-apply
-- safe in case operators run down twice.
DROP TABLE IF EXISTS system_disk_history;
