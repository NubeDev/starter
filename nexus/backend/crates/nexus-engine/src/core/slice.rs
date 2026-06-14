//! Bound a batch's row count by splitting it into `max_batch_rows`-row pieces.
//!
//! The bounded channel between source and sink caps batch *count*, not bytes — a
//! single 100MB batch slips through with green metrics and OOMs the run (roadmap
//! §6 batch-size bound). The pipeline slices every batch to a row ceiling at the
//! source and processor-output boundary, so the channel depth times the ceiling
//! bounds in-flight rows. `RecordBatch::slice` is a zero-copy view over the same
//! buffers, so a batch already within the ceiling is returned untouched and an
//! oversized one is split without copying any column data.

use datafusion::arrow::array::RecordBatch;

/// Split `batch` into chunks of at most `max_rows` rows. A batch already within
/// the ceiling (the overwhelming common case) is returned as a single element
/// with no slicing. `max_rows` is treated as at least 1 by the config parser, so
/// the loop always makes progress.
pub fn slice_to_max(batch: RecordBatch, max_rows: usize) -> Vec<RecordBatch> {
    let rows = batch.num_rows();
    if rows <= max_rows {
        return vec![batch];
    }
    let mut pieces = Vec::with_capacity(rows.div_ceil(max_rows));
    let mut offset = 0;
    while offset < rows {
        let len = max_rows.min(rows - offset);
        pieces.push(batch.slice(offset, len));
        offset += len;
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use datafusion::arrow::array::Int32Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};

    fn batch(n: usize) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("n", DataType::Int32, false)]));
        let values: Vec<i32> = (0..n as i32).collect();
        RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(values))]).unwrap()
    }

    #[test]
    fn within_ceiling_is_untouched() {
        let pieces = slice_to_max(batch(5), 8);
        assert_eq!(pieces.len(), 1, "no split when already within the ceiling");
        assert_eq!(pieces[0].num_rows(), 5);
    }

    #[test]
    fn oversized_splits_into_ceiling_chunks_preserving_total() {
        let pieces = slice_to_max(batch(2050), 1000);
        assert_eq!(pieces.len(), 3, "2050 rows at 1000/chunk → 3 pieces");
        assert_eq!(pieces[0].num_rows(), 1000);
        assert_eq!(pieces[1].num_rows(), 1000);
        assert_eq!(pieces[2].num_rows(), 50, "remainder in the last piece");
        let total: usize = pieces.iter().map(|p| p.num_rows()).sum();
        assert_eq!(total, 2050, "no rows lost or duplicated");
    }

    #[test]
    fn exact_multiple_has_no_empty_trailer() {
        let pieces = slice_to_max(batch(2000), 1000);
        assert_eq!(
            pieces.len(),
            2,
            "exact multiple yields no trailing empty batch"
        );
    }
}
