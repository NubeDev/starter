import 'package:sqlite3/sqlite3.dart';

/// Opens (and migrates) the SQLite file used by the web-mode backend.
///
/// Schema mirrors the Drift definitions in
/// `lib/core/storage/tables/*` — they are kept in lockstep by hand
/// rather than by code-gen, because Drift's web build can't open this
/// file directly anyway (that's the whole reason this server exists).
Database openDatabase(String path) {
  final db = sqlite3.open(path);
  db.execute('PRAGMA journal_mode = WAL;');
  db.execute('PRAGMA foreign_keys = ON;');

  db.execute('''
    CREATE TABLE IF NOT EXISTS connections (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      label TEXT NOT NULL,
      base_url TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT (datetime('now')),
      last_used_at TEXT
    );
  ''');

  db.execute('''
    CREATE TABLE IF NOT EXISTS connection_state (
      id INTEGER PRIMARY KEY DEFAULT 1,
      active_connection_id INTEGER
    );
  ''');

  db.execute('''
    CREATE TABLE IF NOT EXISTS app_settings (
      id INTEGER PRIMARY KEY DEFAULT 1,
      connections_pin TEXT
    );
  ''');

  return db;
}
