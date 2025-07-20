//! Bloom filter implementation using MurmurHash3 (32-bit) hashing.
//! Designed for fast set membership tests with configurable false positive rate.

use bitvec::prelude::*;
use murmur3::murmur3_32::MurmurHasher;
use std::hash::{Hash, Hasher};

/// BloomFilter structure.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: BitVec<u64, Lsb0>,
    k: u32, // number of hash functions
}

impl BloomFilter {
    /// Create a new Bloom filter with `m` bits and `k` hash functions.
    pub fn new(num_bits: usize, k: u32) -> Self {
        let mut bits = BitVec::<u64, Lsb0>::new();
        bits.resize(num_bits, false);
        Self { bits, k }
    }

    fn hash_with_seed<T: Hash>(&self, item: &T, seed: u32) -> usize {
        let mut hasher = MurmurHasher::with_seed(seed);
        item.hash(&mut hasher);
        (hasher.finish() as usize) % self.bits.len()
    }

    /// Insert an item into the filter.
    pub fn insert<T: Hash>(&mut self, item: &T) {
        for i in 0..self.k {
            let idx = self.hash_with_seed(item, i);
            self.bits.set(idx, true);
        }
    }

    /// Check if an item is possibly in the set (false positives possible).
    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        for i in 0..self.k {
            let idx = self.hash_with_seed(item, i);
            if !self.bits[idx] {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::BloomFilter;

    #[test]
    fn basic_insert_and_query() {
        let mut bf = BloomFilter::new(1024, 3);
        bf.insert(&"hello");
        assert!(bf.contains(&"hello"));
        assert!(!bf.contains(&"world"));
    }
} 