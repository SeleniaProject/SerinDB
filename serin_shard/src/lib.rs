//! Sharding algorithms for SerinDB.
use async_trait::async_trait;

#[async_trait]
pub trait ShardRouter: Send + Sync {
    async fn shard_for_key(&self, key: &str) -> u64;
}

pub struct HashRouter {
    shards: u64,
}

impl HashRouter {
    pub fn new(shards: u64) -> Self { Self { shards } }
}

#[async_trait]
impl ShardRouter for HashRouter {
    async fn shard_for_key(&self, key: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut h);
        (h.finish() % self.shards) as u64
    }
}

pub struct RangeRouter {
    ranges: Vec<(String, String, u64)>, // start, end, shard_id
}

impl RangeRouter {
    pub fn new(ranges: Vec<(String, String, u64)>) -> Self { Self { ranges } }
}

#[async_trait]
impl ShardRouter for RangeRouter {
    async fn shard_for_key(&self, key: &str) -> u64 {
        for (start, end, id) in &self.ranges {
            if key >= start && key < end { return *id; }
        }
        0
    }
} 