//! SerinDB transaction layer primitives (MVCC snapshot).
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global monotonically increasing timestamp generator (single node MVP).
static GLOBAL_TS: AtomicU64 = AtomicU64::new(1);

/// Generate next commit timestamp.
pub fn next_ts() -> u64 {
    GLOBAL_TS.fetch_add(1, Ordering::SeqCst)
}

/// A record version stored in MVCC storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionedTuple<T> {
    /// Begin timestamp (inclusive).
    pub min_ts: u64,
    /// End timestamp (exclusive). Running/visible if max_ts = u64::MAX.
    pub max_ts: u64,
    /// Actual tuple payload.
    pub value: T,
}

impl<T> VersionedTuple<T> {
    /// Create new committed tuple visible to future snapshots.
    pub fn new_committed(value: T, ts: u64) -> Self {
        Self {
            min_ts: ts,
            max_ts: u64::MAX,
            value,
        }
    }

    /// Check visibility for snapshot at given timestamp.
    pub fn visible_at(&self, snap_ts: u64) -> bool {
        self.min_ts <= snap_ts && snap_ts < self.max_ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvcc_visibility() {
        let ts1 = next_ts();
        let rec = VersionedTuple::new_committed(10, ts1);
        assert!(rec.visible_at(ts1));
        let ts2 = next_ts();
        assert!(rec.visible_at(ts2));
    }
} 