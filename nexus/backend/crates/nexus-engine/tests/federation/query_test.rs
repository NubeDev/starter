//! Federation seam test: run one read-only SQL statement that JOINs a Parquet
//! file against a CSV file through [`FederatedQuery`], proving the alternate
//! query path resolves each `ds_<alias>` table via DataFusion's own catalog,
//! executes the cross-source join, and honours the output caps (truncation).
//!
//! No database is required to prove the seam — two local files exercise the
//! exact path a registered Postgres datasource takes (register a table under
//! `ds_<alias>`, plan + run, collect under the caps). The Postgres branch is a
//! different `TableProvider` behind the same registration, and its live join is
//! covered by the API-level docker e2e; the engine seam is file-driven so it
//! runs in CI without a broker.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use datafusion::arrow::array::{Int64Array, RecordBatch, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::parquet::arrow::ArrowWriter;
use nexus_engine::{Caps, FederatedQuery, FederatedSource};

/// A scratch directory removed on drop so a failed test never leaks fixtures.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "nexus-federation-test-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Write a two-column `(id, name)` Parquet file the test joins against.
fn write_devices_parquet(path: &std::path::Path, rows: &[(i64, &str)]) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    let ids: Int64Array = rows.iter().map(|(id, _)| *id).collect();
    let names: StringArray = rows.iter().map(|(_, n)| Some(*n)).collect();
    let batch =
        RecordBatch::try_new(schema.clone(), vec![Arc::new(ids), Arc::new(names)]).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

/// Write a `(device_id, temp_c)` CSV file the test joins against.
fn write_readings_csv(path: &std::path::Path, rows: &[(i64, i64)]) {
    let mut text = String::from("device_id,temp_c\n");
    for (device_id, temp_c) in rows {
        text.push_str(&format!("{device_id},{temp_c}\n"));
    }
    std::fs::write(path, text).unwrap();
}

#[tokio::test]
async fn joins_a_parquet_against_a_csv_in_one_sql() {
    let scratch = Scratch::new("join");
    let parquet = scratch.path("devices.parquet");
    let csv = scratch.path("readings.csv");
    write_devices_parquet(&parquet, &[(1, "boiler"), (2, "chiller")]);
    write_readings_csv(&csv, &[(1, 80), (2, 5), (1, 82)]);

    let query = FederatedQuery {
        sql: "SELECT d.name, r.temp_c \
              FROM ds_devices d JOIN ds_readings r ON d.id = r.device_id \
              ORDER BY d.name, r.temp_c"
            .to_string(),
        sources: vec![
            (
                "devices".to_string(),
                FederatedSource::Parquet {
                    path: parquet.display().to_string(),
                },
            ),
            (
                "readings".to_string(),
                FederatedSource::Csv {
                    path: csv.display().to_string(),
                    has_header: true,
                },
            ),
        ],
    };

    let outcome = query
        .run(Caps::new(1000, 1 << 20, Duration::from_secs(30)))
        .await
        .expect("a file-to-file federated join runs to completion");

    assert_eq!(outcome.stats.row_count, 3, "the join produced three rows");
    assert!(!outcome.stats.truncated, "an uncapped result is not truncated");
    let names: Vec<&str> = outcome.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["name", "temp_c"], "schema is the SELECT projection");
    // boiler has two readings (80, 82), chiller one (5); ORDER BY name, temp_c.
    let pairs: Vec<(&str, i64)> = outcome
        .rows
        .iter()
        .map(|r| (r["name"].as_str().unwrap(), r["temp_c"].as_i64().unwrap()))
        .collect();
    assert_eq!(
        pairs,
        [("boiler", 80), ("boiler", 82), ("chiller", 5)],
        "rows arrive shaped + ordered by the federated SQL"
    );
}

#[tokio::test]
async fn row_cap_truncates_a_federated_result() {
    let scratch = Scratch::new("cap");
    let csv = scratch.path("many.csv");
    let rows: Vec<(i64, i64)> = (0..500).map(|i| (i, i)).collect();
    write_readings_csv(&csv, &rows);

    let query = FederatedQuery {
        sql: "SELECT device_id, temp_c FROM ds_readings".to_string(),
        sources: vec![(
            "readings".to_string(),
            FederatedSource::Csv {
                path: csv.display().to_string(),
                has_header: true,
            },
        )],
    };

    let outcome = query
        .run(Caps::rows(10))
        .await
        .expect("a capped federated query still returns an outcome");

    assert!(
        outcome.stats.truncated,
        "hitting the row cap is reported as truncated, not an error"
    );
    assert!(
        outcome.stats.row_count <= 10,
        "the collector stops at the cap rather than buffering all 500 rows"
    );
}

#[tokio::test]
async fn ddl_and_dml_are_rejected_on_the_federated_path() {
    let scratch = Scratch::new("readonly");
    let csv = scratch.path("r.csv");
    write_readings_csv(&csv, &[(1, 1)]);

    let source = || {
        (
            "readings".to_string(),
            FederatedSource::Csv {
                path: csv.display().to_string(),
                has_header: true,
            },
        )
    };

    // A federated query is a literal read; DDL/DML must not plan.
    let drop = FederatedQuery {
        sql: "DROP TABLE ds_readings".to_string(),
        sources: vec![source()],
    };
    assert!(
        drop.run(Caps::unbounded()).await.is_err(),
        "DDL is rejected on the read-only federated path"
    );

    let insert = FederatedQuery {
        sql: "INSERT INTO ds_readings VALUES (9, 9)".to_string(),
        sources: vec![source()],
    };
    assert!(
        insert.run(Caps::unbounded()).await.is_err(),
        "DML is rejected on the read-only federated path"
    );
}
