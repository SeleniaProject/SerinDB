use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use thiserror::Error;

use crate::next_ts;

/// Transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TxnId(pub u64);

/// Lock modes (hierarchical).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    /// Intention Shared.
    IS,
    /// Intention Exclusive.
    IX,
    /// Shared.
    S,
    /// Exclusive.
    X,
}

impl LockMode {
    /// Check compatibility between two lock modes.
    pub fn compatible(self, other: Self) -> bool {
        use LockMode::*;
        matches!((self, other),
            (IS, IS) | (IS, S) | (S, IS) | (IX, IX) if false) // fallback
        || match (self, other) {
            (IS, IS) | (IS, S) | (S, IS) | (S, S) => true,
            _ => false,
        }
    }
}

/// Lock table entry.
#[derive(Default)]
struct LockEntry {
    granted: Vec<(TxnId, LockMode)>,
    waiting: VecDeque<(TxnId, LockMode)>,
}

/// Deadlock error.
#[derive(Debug, Error)]
#[error("deadlock detected for txn {0:?}")]
pub struct DeadlockError(pub TxnId);

/// Simple lock manager with Wait-For Graph deadlock detection.
#[derive(Default)]
pub struct LockManager {
    table: Mutex<HashMap<String, LockEntry>>, // resource-id -> entry
}

impl LockManager {
    /// Acquire a lock, blocking other incompatible holders.
    pub fn lock(&self, txn: TxnId, res: &str, mode: LockMode) -> Result<(), DeadlockError> {
        let mut tbl = self.table.lock().unwrap();
        let entry = tbl.entry(res.to_string()).or_default();
        if entry.granted.iter().all(|&(_, m)| m.compatible(mode)) {
            entry.granted.push((txn, mode));
            return Ok(());
        }
        entry.waiting.push_back((txn, mode));
        drop(tbl);
        // Deadlock detection simplified: if txn waits on itself via graph size > 5 detect.
        if self.detect_deadlock(txn) {
            self.unlock_wait(txn, res);
            return Err(DeadlockError(txn));
        }
        Ok(())
    }

    /// Release all locks held by txn.
    pub fn release_all(&self, txn: TxnId) {
        let mut tbl = self.table.lock().unwrap();
        for entry in tbl.values_mut() {
            entry.granted.retain(|&(t, _)| t != txn);
            entry.waiting.retain(|&(t, _)| t != txn);
        }
    }

    fn unlock_wait(&self, txn: TxnId, res: &str) {
        let mut tbl = self.table.lock().unwrap();
        if let Some(entry) = tbl.get_mut(res) {
            entry.waiting.retain(|&(t, _)| t != txn);
        }
    }

    /// Very naive Wait-For Graph cycle detection.
    fn detect_deadlock(&self, start: TxnId) -> bool {
        let tbl = self.table.lock().unwrap();
        let mut graph: HashMap<TxnId, HashSet<TxnId>> = HashMap::new();
        for entry in tbl.values() {
            if let Some(&(front_txn, _)) = entry.waiting.front() {
                let holders: HashSet<TxnId> = entry.granted.iter().map(|&(t, _)| t).collect();
                graph.entry(front_txn).or_default().extend(holders);
            }
        }
        drop(tbl);
        // BFS to find cycle to start.
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back(start);
        while let Some(txn) = queue.pop_front() {
            if !visited.insert(txn) {
                continue;
            }
            if let Some(neigh) = graph.get(&txn) {
                for &n in neigh {
                    if n == start {
                        return true;
                    }
                    queue.push_back(n);
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_grant_and_deadlock() {
        let lm = LockManager::default();
        let t1 = TxnId(1);
        let t2 = TxnId(2);
        lm.lock(t1, "r1", LockMode::S).unwrap();
        assert!(lm.lock(t2, "r1", LockMode::S).is_ok()); // compatible
        // Deadlock detection path simple simulation
        lm.lock(t1, "r2", LockMode::X).unwrap();
        let res = lm.lock(t2, "r2", LockMode::X);
        assert!(res.is_err());
    }
} 