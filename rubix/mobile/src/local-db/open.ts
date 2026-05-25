// open.ts — opens the SQLite database and runs migrations.
//
// The ONLY file that touches `expo-sqlite` directly (LOCAL-DB.md §File
// layout). Every verb file takes a `SQLiteDatabase` argument so verbs are
// unit-testable against an in-memory db.
//
// Migration ledger lives in `app_state` under the `__migrations.applied`
// key as a JSON array of ids. Kept inside `app_state` (not a separate
// table) so a fresh database needs exactly one migration to apply, the
// one that creates `app_state` itself. Bootstrapping discipline:
//
//   1. Open the db.
//   2. Ensure `app_state` exists (idempotent CREATE TABLE in migration 2).
//   3. Read applied ids.
//   4. Run every un-applied migration in order, recording each.

import * as SQLite from 'expo-sqlite';

import { LocalDbError } from './errors';
import { MIGRATIONS } from './migrations';

/** Stable filename so the same db is opened across cold starts. */
export const DB_NAME = 'rubix.db';

const MIGRATION_LEDGER_KEY = '__migrations.applied';

export type Database = SQLite.SQLiteDatabase;

/**
 * Open the on-device SQLite database and apply any pending migrations.
 * Idempotent — safe to call repeatedly across the React tree thanks to
 * the `LocalDbProvider`'s singleton wrapper.
 */
export async function openDb(name: string = DB_NAME): Promise<Database> {
  let db: Database;
  try {
    db = await SQLite.openDatabaseAsync(name);
  } catch (cause) {
    throw new LocalDbError('open', `failed to open ${name}`, cause);
  }
  await runMigrations(db);
  return db;
}

async function runMigrations(db: Database): Promise<void> {
  // Bootstrap the ledger table first. We piggyback on `app_state` because
  // migration 0002 creates it, but until 0002 runs we have nowhere to
  // record what has been applied. Solution: run 0001 unconditionally on a
  // fresh database (CREATE TABLE IF NOT EXISTS makes it safe on re-open),
  // run 0002 unconditionally to ensure the ledger row exists, THEN read
  // the ledger to decide on 0003+.
  try {
    for (const m of MIGRATIONS.slice(0, 2)) {
      await db.execAsync(m.sql);
    }
  } catch (cause) {
    throw new LocalDbError('migrate', 'bootstrap migrations failed', cause);
  }

  const applied = await readApplied(db);
  // 0001 + 0002 may have been recorded on a prior boot — re-applying the
  // CREATE TABLE IF NOT EXISTS above is harmless either way.
  for (const m of MIGRATIONS) {
    if (applied.has(m.id)) continue;
    try {
      await db.execAsync(m.sql);
      applied.add(m.id);
      await writeApplied(db, applied);
    } catch (cause) {
      throw new LocalDbError('migrate', `migration ${m.id} failed`, cause);
    }
  }
}

async function readApplied(db: Database): Promise<Set<string>> {
  const row = await db.getFirstAsync<{ v: string }>(
    'SELECT v FROM app_state WHERE k = ?',
    [MIGRATION_LEDGER_KEY],
  );
  if (!row?.v) return new Set();
  try {
    const ids = JSON.parse(row.v) as unknown;
    return Array.isArray(ids) ? new Set(ids.filter((x): x is string => typeof x === 'string')) : new Set();
  } catch {
    return new Set();
  }
}

async function writeApplied(db: Database, applied: Set<string>): Promise<void> {
  const json = JSON.stringify([...applied].sort());
  await db.runAsync(
    'INSERT INTO app_state (k, v) VALUES (?, ?) ON CONFLICT(k) DO UPDATE SET v = excluded.v',
    [MIGRATION_LEDGER_KEY, json],
  );
}
