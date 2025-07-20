//! Raft consensus layer for SerinDB cluster.
use openraft::{Config, Raft, RaftNetwork, RaftStorage, AppData, AppDataResponse};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry(pub Vec<u8>);
impl AppData for LogEntry {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientResp;
impl AppDataResponse for ClientResp {}

pub type NodeId = u64;

pub struct Network;
#[async_trait]
impl RaftNetwork<LogEntry> for Network {}

pub struct Storage;
#[async_trait]
impl RaftStorage<LogEntry, ClientResp> for Storage {}

pub type SerinRaft = Raft<LogEntry, ClientResp, Network, Storage>;

pub fn new_raft(node_id: NodeId) -> SerinRaft {
    let cfg = Config::build("serin-cluster").validate().unwrap();
    let net = Network;
    let store = Storage;
    Raft::new(node_id, Arc::new(cfg), net, store)
} 