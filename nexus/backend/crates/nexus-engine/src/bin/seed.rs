//! Bulk-seed the three simulator profiles into a datasource Postgres for
//! testing — the static-fixture counterpart to the live `simulator` input. It
//! reuses the same row builder ([`sim::build_row`]), so seeded rows match what a
//! `simulator → postgres` flow would write.
//!
//! Usage:
//!
//! ```text
//! DATABASE_URL=postgres://… nexus-seed [rows_per_profile]
//! ```
//!
//! Creates `sim_hvac`, `sim_energy`, `sim_door` if absent, then inserts
//! `rows_per_profile` (default 200) deterministic rows into each. Timestamps
//! march backwards from now at one-minute steps so the rows form a plausible
//! recent history.

use std::sync::atomic::AtomicU64;

use chrono::{Duration, Utc};
use nexus_engine::source::sim::{self, Profile};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const DEFAULT_ROWS: usize = 200;
const PROFILES: [Profile; 3] = [Profile::Hvac, Profile::Energy, Profile::Door];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = std::env::var("DATABASE_URL")
        .map_err(|_| "set DATABASE_URL to the datasource Postgres connection string")?;
    let rows: usize = std::env::args()
        .nth(1)
        .map(|a| a.parse())
        .transpose()
        .map_err(|_| "rows_per_profile must be a number")?
        .unwrap_or(DEFAULT_ROWS);

    let pool = PgPoolOptions::new().max_connections(2).connect(&uri).await?;

    for profile in PROFILES {
        let n = seed_profile(&pool, profile, rows).await?;
        println!("seeded {n} rows into {}", profile.table());
    }
    Ok(())
}

/// Create the profile's table if needed and insert `rows` deterministic rows.
/// Each profile uses a fixed seed and device id so re-running yields the same
/// data (after a truncate). Returns the number of rows inserted.
async fn seed_profile(pool: &PgPool, profile: Profile, rows: usize) -> Result<usize, sqlx::Error> {
    sqlx::query(&create_table_sql(profile)).execute(pool).await?;

    // Fixed per-profile seed → repeatable stream; one device per profile.
    let state = sim::seed_state(profile as u64 + 1);
    let kwh = AtomicU64::new(0);
    let device_id = format!("sim-{}", profile.table());
    let now = Utc::now();

    for i in 0..rows {
        // March timestamps backwards so row 0 is oldest, last row is "now".
        let ts = (now - Duration::minutes((rows - 1 - i) as i64)).to_rfc3339();
        let row = sim::build_row(profile, &device_id, &ts, &state, &kwh);
        insert_row(pool, profile.table(), &row).await?;
    }
    Ok(rows)
}

/// `CREATE TABLE IF NOT EXISTS` for a profile, columns matching its row shape.
fn create_table_sql(profile: Profile) -> String {
    let cols = match profile {
        Profile::Hvac => {
            "temp_c double precision, setpoint double precision, fan_speed double precision"
        }
        Profile::Energy => "kwh_total double precision, power_w double precision",
        Profile::Door => "open boolean, zone text",
    };
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\
            id bigserial PRIMARY KEY, device_id text NOT NULL, ts timestamptz NOT NULL, {cols})",
        profile.table()
    )
}

/// Insert one JSON row, binding each column by name. Mirrors the postgres
/// sink's bound-parameter insert; values are never concatenated into SQL.
async fn insert_row(pool: &PgPool, table: &str, row: &Value) -> Result<(), sqlx::Error> {
    let obj = row.as_object().expect("build_row returns an object");
    let cols: Vec<&String> = obj.keys().collect();
    let column_list = cols
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!(
        "INSERT INTO {table} ({column_list}) "
    ));
    qb.push_values(std::iter::once(()), |mut b, _| {
        for c in &cols {
            bind_value(&mut b, &obj[*c]);
        }
    });
    qb.build().execute(pool).await?;
    Ok(())
}

/// Bind one JSON value to its natural Postgres type. `ts` arrives as an RFC3339
/// string and is bound as a `timestamptz`; everything else maps from the JSON
/// scalar.
fn bind_value<'a>(
    b: &mut sqlx::query_builder::Separated<'_, 'a, sqlx::Postgres, &'static str>,
    v: &'a Value,
) {
    match v {
        Value::Bool(x) => {
            b.push_bind(*x);
        }
        Value::Number(n) if n.is_i64() => {
            b.push_bind(n.as_i64().unwrap());
        }
        Value::Number(n) => {
            b.push_bind(n.as_f64().unwrap());
        }
        Value::String(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => {
                b.push_bind(dt.with_timezone(&Utc));
            }
            Err(_) => {
                b.push_bind(s.clone());
            }
        },
        _ => {
            b.push_bind::<Option<String>>(None);
        }
    }
}
