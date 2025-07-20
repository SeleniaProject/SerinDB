use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use hdrhistogram::Histogram;
use serde::{Serialize, Deserialize};
use bytes::BufMut;

/// Logical identifier for each Data Center.
pub type DcId = u8;

/// WAL sequence number.
pub type Lsn = u64;

/// Single WAL payload frame transferred between DCs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub dc_id: DcId,
    pub lsn: Lsn,
    pub timestamp_ns: u64,
    pub payload: Vec<u8>,
}

/// Conflict resolution based on Lamport timestamps + DC precedence.
pub fn resolve_conflict(local: &LogEntry, remote: &LogEntry) -> bool {
    if remote.lsn > local.lsn {
        return true;
    }
    if remote.lsn == local.lsn {
        // Tie-break with DC id (lower wins)
        return remote.dc_id < local.dc_id;
    }
    false
}

/// Aggregated replication metrics.
#[derive(Default)]
pub struct Metrics {
    pub latency_hist: Mutex<Histogram<u64>>, // ns
}

impl Metrics {
    pub fn new() -> Self {
        let hist = Histogram::new(3).expect("hist");
        Metrics { latency_hist: Mutex::new(hist) }
    }
}

/// Asynchronous replication channel server.
pub struct ReplicationServer {
    address: String,
    dc_id: DcId,
    storage: Arc<dyn ReplicatedStore + Send + Sync>,
    metrics: Arc<Metrics>,
}

#[async_trait::async_trait]
pub trait ReplicatedStore {
    async fn append_entry(&self, entry: LogEntry) -> Result<()>;
}

impl ReplicationServer {
    pub fn new<A: Into<String>>(addr: A, dc_id: DcId, storage: Arc<dyn ReplicatedStore + Send + Sync>) -> Self {
        Self { address: addr.into(), dc_id, storage, metrics: Arc::new(Metrics::new()) }
    }

    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(&self.address).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let storage = self.storage.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, storage, metrics).await {
                    eprintln!("replication connection error: {e}");
                }
            });
        }
    }
}

async fn handle_connection(mut stream: TcpStream, storage: Arc<dyn ReplicatedStore + Send + Sync>, metrics: Arc<Metrics>) -> Result<()> {
    let mut len_buf = [0u8; 4];
    loop {
        if stream.read_exact(&mut len_buf).await.is_err() { break; }
        let frame_len = u32::from_be_bytes(len_buf) as usize;
        let mut frame = vec![0u8; frame_len];
        stream.read_exact(&mut frame).await?;
        let entry: LogEntry = serde_json::from_slice(&frame)?;
        let start = tokio::time::Instant::now();
        storage.append_entry(entry).await?;
        let latency = start.elapsed().as_nanos() as u64;
        let mut hist = metrics.latency_hist.lock().await;
        let _ = hist.record(latency);
    }
    Ok(())
}

/// Replication client pushing logs to a remote DC.
pub struct ReplicationClient {
    peer_addr: String,
    stream: Mutex<Option<TcpStream>>,
    dc_id: DcId,
}

impl ReplicationClient {
    pub fn new<A: Into<String>>(peer: A, dc_id: DcId) -> Self { Self { peer_addr: peer.into(), stream: Mutex::new(None), dc_id } }

    async fn get_stream(&self) -> Result<TcpStream> {
        let mut guard = self.stream.lock().await;
        if let Some(ref mut s) = *guard { return Ok(s.clone()); }
        let s = TcpStream::connect(&self.peer_addr).await?;
        *guard = Some(s.clone());
        Ok(s)
    }

    /// Send a WAL payload to remote DC.
    pub async fn send(&self, lsn: Lsn, payload: &[u8]) -> Result<()> {
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
        let entry = LogEntry { dc_id: self.dc_id, lsn, timestamp_ns: ts, payload: payload.to_vec() };
        let data = serde_json::to_vec(&entry)?;
        let mut stream = self.get_stream().await?;
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.put_u32(data.len() as u32);
        buf.extend_from_slice(&data);
        stream.write_all(&buf).await?;
        Ok(())
    }
}

/// In-memory replicated store for demo purposes.
pub struct MemoryStore {
    entries: Mutex<HashMap<Lsn, LogEntry>>,
}

impl MemoryStore { pub fn new() -> Self { Self { entries: Mutex::new(HashMap::new()) } } }

#[async_trait::async_trait]
impl ReplicatedStore for MemoryStore {
    async fn append_entry(&self, entry: LogEntry) -> Result<()> {
        let mut map = self.entries.lock().await;
        match map.get(&entry.lsn) {
            Some(local) if !resolve_conflict(local, &entry) => return Ok(()),
            _ => { map.insert(entry.lsn, entry); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn conflict_resolution() {
        let a = LogEntry { dc_id: 1, lsn: 10, timestamp_ns: 1, payload: vec![] };
        let b = LogEntry { dc_id: 2, lsn: 10, timestamp_ns: 2, payload: vec![] };
        assert!(resolve_conflict(&a, &b));
    }
} 