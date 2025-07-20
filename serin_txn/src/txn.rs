use crate::gtm::Gtm;
use crate::lock::{LockManager, LockMode, TxnId};
use crate::VersionedTuple;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Transaction status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnStatus {
    /// Active running.
    Active,
    /// Prepared (phase1 complete).
    Prepared,
    /// Committed.
    Committed,
    /// Aborted.
    Aborted,
}

/// Prepare log entry persisted to WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRecord {
    pub txn_id: TxnId,
    pub commit_ts: u64,
}

/// Simple transaction manager supporting single-node 2PC.
pub struct TxnManager {
    gtm: Gtm,
    lock_mgr: Arc<LockManager>,
    statuses: Mutex<HashMap<TxnId, TxnStatus>>, // for test only
}

impl Default for TxnManager {
    fn default() -> Self {
        Self {
            gtm: Gtm::default(),
            lock_mgr: Arc::new(LockManager::default()),
            statuses: Mutex::new(HashMap::new()),
        }
    }
}

impl TxnManager {
    /// Begin a new transaction, returning its id.
    pub fn begin(&self) -> TxnId {
        let id = TxnId(self.gtm.alloc());
        self.statuses.lock().unwrap().insert(id, TxnStatus::Active);
        id
    }

    /// Acquire exclusive lock on resource (table-level for MVP).
    pub fn lock_x(&self, txn: TxnId, res: &str) -> bool {
        self.lock_mgr.lock(txn, res, LockMode::X).is_ok()
    }

    /// Prepare phase – persists PrepareRecord (mock: return struct).
    pub fn prepare(&self, txn: TxnId) -> PrepareRecord {
        let ts = self.gtm.alloc();
        self.statuses.lock().unwrap().insert(txn, TxnStatus::Prepared);
        PrepareRecord { txn_id: txn, commit_ts: ts }
    }

    /// Commit after prepare (phase2).
    pub fn commit(&self, txn: TxnId) {
        self.statuses.lock().unwrap().insert(txn, TxnStatus::Committed);
        self.lock_mgr.release_all(txn);
    }

    /// Crash recovery that marks prepared txns as committed.
    pub fn recover(&self, prepare_records: &[PrepareRecord]) {
        let mut statuses = self.statuses.lock().unwrap();
        for rec in prepare_records {
            statuses.insert(rec.txn_id, TxnStatus::Committed);
        }
    }

    /// Get status (for tests).
    pub fn status(&self, txn: TxnId) -> TxnStatus {
        *self.statuses.lock().unwrap().get(&txn).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_phase_commit_flow() {
        let tm = TxnManager::default();
        let txn = tm.begin();
        assert!(tm.lock_x(txn, "t1"));
        let prep = tm.prepare(txn);
        // simulate crash before commit – store prepare log
        let recovered_tm = TxnManager::default();
        recovered_tm.recover(&[prep]);
        assert_eq!(recovered_tm.status(txn), TxnStatus::Committed);
    }
} 