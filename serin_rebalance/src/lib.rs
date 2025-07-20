//! Automatic rebalancer using 2-dimensional bin packing.
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStat {
    pub id: String,
    pub read_qps: f64,
    pub write_qps: f64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPlacement {
    pub shard_id: u64,
    pub target_node: String,
}

/// Score function weight for write traffic.
const WRITE_WEIGHT: f64 = 2.0;

/// Perform a naive 2D bin packing rebalancing.
/// Returns a vector of shard move plans.
///
/// capacity_qps and capacity_bytes reflect per-node thresholds.
pub fn rebalance(nodes: &[NodeStat], shards: &[(u64, f64, u64)], capacity_qps: f64, capacity_bytes: u64) -> Vec<ShardPlacement> {
    let mut plans = Vec::new();
    // Sort shards descending by composite weight.
    let mut sorted = shards.to_vec();
    sorted.sort_by(|a, b| {
        let wa = a.1 * WRITE_WEIGHT + a.2 as f64 / 1_000_000.0;
        let wb = b.1 * WRITE_WEIGHT + b.2 as f64 / 1_000_000.0;
        wb.partial_cmp(&wa).unwrap()
    });
    // Greedy fit.
    let mut node_load: Vec<(f64, u64)> = nodes.iter().map(|n| (n.read_qps + n.write_qps * WRITE_WEIGHT, n.used_bytes)).collect();
    for (shard_id, qps, bytes) in sorted {
        for (idx, (lqps, lbytes)) in node_load.iter_mut().enumerate() {
            if *lqps + qps <= capacity_qps && *lbytes + bytes <= capacity_bytes {
                *lqps += qps;
                *lbytes += bytes;
                plans.push(ShardPlacement { shard_id, target_node: nodes[idx].id.clone() });
                break;
            }
        }
    }
    plans
} 