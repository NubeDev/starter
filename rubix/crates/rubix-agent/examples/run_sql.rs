//! Throwaway SQL-file runner (no local psql in this env).
//!
//!   RUBIX_PROBE_DSN=postgres://... \
//!     cargo run -p rubix-agent --example run_sql -- path/to/file.sql
//!
//! Executes the whole file as one batch inside a single transaction
//! (`raw_sql` supports multiple statements + DO blocks). Pass
//! `--dry-run` to parse/connect only.

use sqlx::Executor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut path: Option<String> = None;
    let mut dry = false;
    for a in args.by_ref() {
        match a.as_str() {
            "--dry-run" => dry = true,
            other => path = Some(other.to_owned()),
        }
    }
    let path = path.ok_or_else(|| anyhow::anyhow!("usage: run_sql <file.sql> [--dry-run]"))?;
    let sql = std::fs::read_to_string(&path)?;

    let dsn = std::env::var("RUBIX_PROBE_DSN")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&dsn)
        .await?;

    if dry {
        println!("[dry-run] connected OK; {} bytes of SQL not executed", sql.len());
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    tx.execute(sqlx::raw_sql(&sql)).await?;
    tx.commit().await?;
    println!("[ok] executed {} ({} bytes) in one transaction", path, sql.len());
    pool.close().await;
    Ok(())
}
