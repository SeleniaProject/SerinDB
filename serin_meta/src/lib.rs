//! Cluster metadata service with ShardMap gRPC API.
use async_trait::async_trait;
use openraft::Raft;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMapEntry {
    pub shard_id: u64,
    pub node: String,
}

#[derive(Default)]
pub struct ShardMapStore {
    inner: tokio::sync::RwLock<HashMap<u64, String>>,
}

impl ShardMapStore {
    pub async fn get(&self, id: u64) -> Option<String> { self.inner.read().await.get(&id).cloned() }
    pub async fn set(&self, id: u64, node: String) { self.inner.write().await.insert(id, node); }
}

pub mod proto {
    tonic::include_proto!("serin.meta");
}

use proto::shard_map_server::{ShardMap, ShardMapServer};
use proto::{GetRequest, GetResponse, UpdateRequest, UpdateResponse};

pub fn service(store: Arc<ShardMapStore>) -> ShardMapServer<MyService> { ShardMapServer::new(MyService { store }) }

pub struct MyService {
    store: Arc<ShardMapStore>,
}

#[async_trait]
impl ShardMap for MyService {
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let id = request.into_inner().shard_id;
        let node = self.store.get(id).await.unwrap_or_default();
        Ok(Response::new(GetResponse { node }))
    }

    async fn update(&self, request: Request<UpdateRequest>) -> Result<Response<UpdateResponse>, Status> {
        let req = request.into_inner();
        self.store.set(req.shard_id, req.node).await;
        Ok(Response::new(UpdateResponse {}))
    }
} 