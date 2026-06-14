//! Throwaway read-only probe: run a query, print rows as TSV.
//!
//!   RUBIX_PROBE_DSN=postgres://... \
//!     cargo run -p rubix-agent --example pg_probe -- "SELECT 1"
//!
//! Each CLI arg is one statement; results are printed with a blank
//! line between statements. Read-only by convention — for inspection.

use sqlx::{Column, Row};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let queries: Vec<String> = std::env::args().skip(1).collect();
    let dsn = std::env::var("RUBIX_PROBE_DSN")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(15))
        .connect(&dsn)
        .await?;

    for q in &queries {
        println!("### {q}");
        match sqlx::query(q).fetch_all(&pool).await {
            Ok(rows) => {
                if let Some(first) = rows.first() {
                    let header: Vec<String> =
                        first.columns().iter().map(|c| c.name().to_owned()).collect();
                    println!("{}", header.join("\t"));
                }
                for row in &rows {
                    let mut cells = Vec::new();
                    for i in 0..row.len() {
                        // Try common types, fall back to NULL/"?".
                        let s = row
                            .try_get::<String, _>(i)
                            .or_else(|_| row.try_get::<i64, _>(i).map(|v| v.to_string()))
                            .or_else(|_| row.try_get::<i32, _>(i).map(|v| v.to_string()))
                            .or_else(|_| row.try_get::<f64, _>(i).map(|v| v.to_string()))
                            .or_else(|_| row.try_get::<bool, _>(i).map(|v| v.to_string()))
                            .unwrap_or_else(|_| "<?>".to_string());
                        cells.push(s);
                    }
                    println!("{}", cells.join("\t"));
                }
                println!("({} rows)\n", rows.len());
            }
            Err(e) => println!("ERROR: {e}\n"),
        }
    }
    pool.close().await;
    Ok(())
}
