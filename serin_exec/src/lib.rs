//! SerinDB vectorized execution primitives (MVP).
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Number of rows per column batch (MVP value).
pub const BATCH_CAPACITY: usize = 4096;

/// Column batch storing homogeneous type `i64` for MVP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnBatch {
    /// Values buffer (length <= BATCH_CAPACITY).
    pub values: Vec<i64>,
}

impl ColumnBatch {
    /// Create empty batch.
    pub fn new() -> Self {
        Self { values: Vec::with_capacity(BATCH_CAPACITY) }
    }

    /// Push a value, returning false if batch full.
    pub fn push(&mut self, v: i64) -> bool {
        if self.values.len() >= BATCH_CAPACITY {
            return false;
        }
        self.values.push(v);
        true
    }

    /// Simple vectorized filter using predicate closure.
    pub fn filter(&self, pred: impl Fn(i64) -> bool) -> ColumnBatch {
        let mut out = ColumnBatch::new();
        // naive loop; placeholder for SIMD acceleration.
        for &v in &self.values {
            if pred(v) {
                out.values.push(v);
            }
        }
        out
    }
}

#[cfg(feature = "jit")]
pub mod jit;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_push_and_filter() {
        let mut batch = ColumnBatch::new();
        for i in 0..100 {
            assert!(batch.push(i));
        }
        let even = batch.filter(|v| v % 2 == 0);
        assert_eq!(even.values.len(), 50);
    }
} 