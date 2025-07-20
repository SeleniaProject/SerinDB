use std::sync::atomic::{AtomicU64, Ordering};

/// Global Transaction Manager issuing monotonic timestamps.
#[derive(Debug)]
pub struct Gtm {
    counter: AtomicU64,
}

impl Default for Gtm {
    fn default() -> Self {
        Self {
            counter: AtomicU64::new(1),
        }
    }
}

impl Gtm {
    /// Allocate a new monotonically increasing timestamp.
    #[inline]
    pub fn alloc(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn allocate_one_million_fast() {
        let gtm = Gtm::default();
        let start = Instant::now();
        for _ in 0..1_000_000 {
            gtm.alloc();
        }
        let elapsed = start.elapsed();
        // Ensure throughput >1M per second (i.e., <1s for 1M).
        assert!(elapsed.as_secs_f64() < 1.0, "allocation too slow: {elapsed:?}");
    }
} 